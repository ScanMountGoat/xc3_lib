// Ported from https://github.com/ScanMountGoat/Smush-Material-Research
// TODO: make dependencies and annotation into a library?
use std::collections::{BTreeMap, HashMap};

use glsl::{
    syntax::{ArraySpecifierDimension, Expr, FunIdentifier, SimpleStatement, TranslationUnit},
    transpiler::glsl::show_expr,
    visitor::{Host, Visit, Visitor},
};

#[derive(Debug, Default)]
struct AssignmentVisitor {
    assignments: Vec<AssignmentDependency>,

    // Cache the last line where each variable was assigned.
    last_assignment_index: HashMap<String, usize>,
}

impl AssignmentVisitor {
    fn add_assignment(&mut self, output: String, input: &Expr, statement: &SimpleStatement) {
        let mut input_last_assignments = Vec::new();
        add_vars(
            input,
            &mut input_last_assignments,
            &self.last_assignment_index,
            None,
        );

        let assignment = AssignmentDependency {
            output,
            input_last_assignments,
            input_expr: input.clone(),
            full_statement: statement.clone(),
        };
        // The visitor doesn't track line numbers.
        // We only need to look up the assignments, so use the index instead.
        self.last_assignment_index
            .insert(assignment.output.clone(), self.assignments.len());
        self.assignments.push(assignment);
    }
}

#[derive(Debug, Clone)]
struct AssignmentDependency {
    output: String,

    full_statement: SimpleStatement,
    input_expr: Expr,

    // Include where any inputs were last initialized.
    // This makes edge traversal O(1) later.
    // Also store color channels from dot expressions like "ZW".
    input_last_assignments: Vec<(LastAssignment, Option<String>)>,
}

#[derive(Debug, Clone)]
enum LastAssignment {
    LineNumber(usize),
    Global(String),
}

pub fn texture_dependencies(translation_unit: &TranslationUnit, var: &str) -> Vec<String> {
    let dependencies = source_dependencies(translation_unit, var);

    // TODO: Rework this later to make fewer assumptions about the code structure.
    // TODO: Rework this to be cleaner and add more tests.
    dependencies
        .iter()
        .filter_map(|(i, d)| {
            texture_identifier_name(&d.input_expr).map(|tex| {
                // Get the initial channels used for the texture function call.
                // This defines the possible channels if we assume one access per texture.
                let mut channels = d.input_last_assignments[0].1.as_ref().unwrap().clone();
                // If only a single channel is accessed initially, there's nothing more to do.
                if channels.len() > 1 {
                    channels = actual_channels(*i, &dependencies, &channels);
                }

                tex + "." + &channels
            })
        })
        .collect()
}

fn actual_channels(
    i: usize,
    dependencies: &BTreeMap<usize, AssignmentDependency>,
    channels: &str,
) -> String {
    // Track which channels are accessed later.
    let mut has_channel = [false; 4];

    // TODO: Find a simpler way to do this.
    // Get the channels accessed on lines using this texture value.
    // We'll assume that the next accesses are single channel for now.
    // Example: a = texture(tex, vec2(0.0)).zw; b = a.y;
    // This allows us to avoid complicated graph traversal.
    // TODO: Is it worth properly collecting and reducing all channel operations?

    // TODO: We only need to search starting from index i?
    // TODO: Is there a faster way to do this?
    for (_, second_channels) in dependencies.iter().flat_map(|d| {
        d.1.input_last_assignments
            .iter()
            .filter(|a| matches!(a.0, LastAssignment::LineNumber(line) if line == i))
    }) {
        if let Some(second_channels) = second_channels {
            for c in second_channels.chars() {
                match c {
                    'x' => has_channel[0] = true,
                    'y' => has_channel[1] = true,
                    'z' => has_channel[2] = true,
                    'w' => has_channel[3] = true,
                    _ => todo!(),
                }
            }
        }
    }

    // Only include the channels that are used later.
    // Each of the channels accesses one of the original channels.
    // a.yz.x is accessing the first channel from yz.
    channels
        .chars()
        .zip(has_channel)
        .filter_map(|(c, is_present)| is_present.then_some(c))
        .collect()
}

// TODO: Return option instead?
fn texture_identifier_name(expr: &Expr) -> Option<String> {
    // Assume textures are only accessed in statements with a single texture function.
    // Accesses may have channels like "texture(the_tex, vec2(0.5)).rgb".
    match expr {
        Expr::FunCall(id, es) => {
            if matches!(id, FunIdentifier::Identifier(fun_id) if fun_id.0.contains("texture")) {
                match &es[0] {
                    Expr::Variable(id) => Some(id.0.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }
        Expr::Dot(e, _) => texture_identifier_name(e),
        _ => None,
    }
}

fn add_vars(
    expr: &Expr,
    vars: &mut Vec<(LastAssignment, Option<String>)>,
    most_recent_assignment: &HashMap<String, usize>,
    channel: Option<&String>,
) {
    // Collect and variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    // TODO: Include constants?
    match expr {
        Expr::Variable(i) => {
            // The base case is a single variable like temp_01.
            // Also handle values like buffer or texture names.
            let assignment = match most_recent_assignment.get(&i.0) {
                Some(i) => LastAssignment::LineNumber(*i),
                None => LastAssignment::Global(i.0.clone()),
            };
            vars.push((assignment, channel.cloned()));
        }
        Expr::IntConst(_) => (),
        Expr::UIntConst(_) => (),
        Expr::BoolConst(_) => (),
        Expr::FloatConst(_) => (),
        Expr::DoubleConst(_) => (),
        Expr::Unary(_, e) => add_vars(e, vars, most_recent_assignment, channel),
        Expr::Binary(_, lh, rh) => {
            add_vars(lh, vars, most_recent_assignment, channel);
            add_vars(rh, vars, most_recent_assignment, channel);
        }
        Expr::Ternary(a, b, c) => {
            add_vars(a, vars, most_recent_assignment, channel);
            add_vars(b, vars, most_recent_assignment, channel);
            add_vars(c, vars, most_recent_assignment, channel);
        }
        Expr::Assignment(_, _, _) => todo!(),
        Expr::Bracket(e, specifier) => {
            // Expressions like array[index] depend on index.
            // TODO: Do we also need to depend on array itself?
            add_vars(e, vars, most_recent_assignment, channel);

            for dim in &specifier.dimensions {
                if let ArraySpecifierDimension::ExplicitlySized(e) = dim {
                    add_vars(e, vars, most_recent_assignment, channel);
                }
            }
        }
        Expr::FunCall(_, es) => {
            for e in es {
                add_vars(e, vars, most_recent_assignment, channel);
            }
        }
        Expr::Dot(e, channel) => {
            // Track the channels accessed by expressions like "value.rgb".
            add_vars(e, vars, most_recent_assignment, Some(&channel.0))
        }
        Expr::PostInc(e) => add_vars(e, vars, most_recent_assignment, channel),
        Expr::PostDec(e) => add_vars(e, vars, most_recent_assignment, channel),
        Expr::Comma(_, _) => todo!(),
    }
}

fn print_expr(expr: &Expr) -> String {
    let mut text = String::new();
    show_expr(&mut text, expr);
    text
}

impl Visitor for AssignmentVisitor {
    fn visit_simple_statement(&mut self, statement: &SimpleStatement) -> Visit {
        match statement {
            SimpleStatement::Expression(Some(glsl::syntax::Expr::Assignment(lh, _, rh))) => {
                let output = print_expr(lh);
                self.add_assignment(output, rh, statement);
                Visit::Children
            }
            SimpleStatement::Declaration(glsl::syntax::Declaration::InitDeclaratorList(l)) => {
                // TODO: is it worth handling complex initializers?
                if let Some(glsl::syntax::Initializer::Simple(init)) = l.head.initializer.as_ref() {
                    let output = l.head.name.as_ref().unwrap().0.clone();
                    self.add_assignment(output, init, statement);
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }
}

fn source_dependencies(
    translation_unit: &TranslationUnit,
    var: &str,
) -> BTreeMap<usize, AssignmentDependency> {
    // Visit each assignment to establish data dependencies.
    // This converts the code to a directed acyclic graph (DAG).
    let mut visitor = AssignmentVisitor::default();
    translation_unit.visit(&mut visitor);

    // Find the last assignment containing the desired variable.
    if let Some((assignment_index, assignment)) = visitor
        .assignments
        .iter()
        .enumerate()
        .rfind(|(_, a)| a.output == var)
    {
        // Follow data dependencies backwards to find all relevant lines.
        let mut dependencies = [(assignment_index, assignment.clone())].into();
        add_dependencies(&mut dependencies, assignment, &visitor.assignments);

        // Sort by line number and remove duplicates.
        // dependencies.sort_by_key(|(i, _)| *i);
        // dependencies.dedup_by_key(|(i, _)| *i);
        dependencies
    } else {
        // Variables not part of the code should have no dependencies.
        BTreeMap::new()
    }
}

fn add_dependencies(
    dependencies: &mut BTreeMap<usize, AssignmentDependency>,
    assignment: &AssignmentDependency,
    assignments: &[AssignmentDependency],
) {
    // Recursively collect lines that the given assignment depends on.
    for (assignment, _) in &assignment.input_last_assignments {
        match assignment {
            LastAssignment::LineNumber(line) => {
                let last_assignment = &assignments[*line];
                dependencies.insert(*line, last_assignment.clone());

                add_dependencies(dependencies, last_assignment, assignments);
            }
            LastAssignment::Global(_) => {
                // TODO: How to handle this case?
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use glsl::{parser::Parse, syntax::ShaderStage, transpiler::glsl::show_simple_statement};
    use indoc::indoc;

    fn print_statement(statement: &SimpleStatement) -> String {
        // TODO: Find a way to pretty print instead?
        let mut text = String::new();
        show_simple_statement(&mut text, statement);
        text
    }

    fn source_dependencies_glsl(source: &str, var: &str) -> String {
        let translation_unit = ShaderStage::parse(source).unwrap();
        let dependencies = source_dependencies(&translation_unit, var);

        // Combine all the lines into source code again.
        // These won't exactly match the originals due to formatting differences.
        dependencies
            .iter()
            .map(|(_, d)| print_statement(&d.full_statement))
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn source_dependencies_final_assignment() {
        let glsl = indoc! {"
            layout (binding = 9, std140) uniform fp_c9
            {
                vec4 fp_c9_data[0x1000];
            };

            void main() 
            {
                float a = fp_c9_data[0].x;
                float b = 2.0;
                float c = a * b;
                float d = fma(a, b, c);
                d = d + 1.0;
                OUT_Color.x = c + d;
            }
        "};

        assert_eq!(
            indoc! {"
                float a = fp_c9_data[0].x;
                float b = 2.;
                float c = a*b;
                float d = fma(a, b, c);
                d = d+1.;
                OUT_Color.x = c+d;
            "},
            source_dependencies_glsl(glsl, "OUT_Color.x")
        );
    }

    #[test]
    fn source_dependencies_intermediate_assignment() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float b = 2.0;
                float d = fma(a, b, -1.0);
                float c = 2 * b;
                d = d + 1.0;
                OUT_Color.x = c + d;
            }
        "};

        assert_eq!(
            indoc! {"
                float b = 2.;
                float c = 2*b;
            "},
            source_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn source_dependencies_type_casts() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 0.0;
                uint b = uint(a) >> 2;
                float d = 3.0 + a;
                float c = data[int(b)];
            }
        "};

        assert_eq!(
            indoc! {"
                float a = 0.;
                uint b = uint(a)>>2;
                float c = data[int(b)];
            "},
            source_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn source_dependencies_missing() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 0.0;
            }
        "};

        assert_eq!("", source_dependencies_glsl(glsl, "d"));
    }

    #[test]
    fn source_dependencies_textures() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float b = texture(texture1, vec2(a + 2.0, 1.0)).x;
                float c = data[int(b)];
            }
        "};

        assert_eq!(
            indoc! {"
                float a = 1.;
                float b = texture(texture1, vec2(a+2., 1.)).x;
                float c = data[int(b)];
            "},
            source_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn texture_dependencies_single_channel() {
        let glsl = indoc! {"
            void main() 
            {
                float t = 1.0;
                float a = texture(texture1, vec2(t)).xw;
                float b = a.y * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(&glsl).unwrap();
        assert_eq!(
            vec!["texture1.w".to_string()],
            texture_dependencies(&tu, "b")
        );
    }

    #[test]
    fn texture_dependencies_single_channel_scalar() {
        let glsl = indoc! {"
            void main() 
            {
                float t = 1.0;
                float a = texture(texture1, vec2(t)).z;
                float b = a * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(&glsl).unwrap();
        assert_eq!(
            vec!["texture1.z".to_string()],
            texture_dependencies(&tu, "b")
        );
    }

    #[test]
    fn texture_dependencies_multiple_channels() {
        let glsl = indoc! {"
            void main() 
            {
                float a = texture(texture1, vec2(1.0)).zw;
                float b = a.y + a.x;
            }
        "};

        let tu = TranslationUnit::parse(&glsl).unwrap();
        assert_eq!(
            vec!["texture1.zw".to_string()],
            texture_dependencies(&tu, "b")
        );
    }
}
