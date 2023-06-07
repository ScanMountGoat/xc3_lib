// Ported from https://github.com/ScanMountGoat/Smush-Material-Research
// TODO: make dependencies and annotation into a library?
use std::collections::{BTreeSet, HashMap};

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
    fn add_assignment(&mut self, output: String, input: &Expr) {
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

struct LineDependencies {
    dependent_lines: BTreeSet<usize>,
    assignments: Vec<AssignmentDependency>,
}

#[derive(Debug, PartialEq)]
pub enum SourceInput {
    Constant(f32),
    Buffer {
        name: String,
        index: usize,
        channels: String,
    },
    Texture {
        name: String,
        channels: String,
    },
}

// TODO: Is it worth converting to string just to parse again in an application?
impl std::fmt::Display for SourceInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceInput::Constant(c) => write!(f, "{c}"),
            SourceInput::Buffer {
                name,
                index,
                channels,
            } => write!(f, "{name}[{index}].{channels}"),
            SourceInput::Texture { name, channels } => write!(f, "{name}.{channels}"),
        }
    }
}

pub fn input_dependencies(translation_unit: &TranslationUnit, var: &str) -> Vec<SourceInput> {
    line_dependencies(translation_unit, var)
        .map(|line_dependencies| {
            // TODO: Rework this later to make fewer assumptions about the code structure.
            // TODO: Rework this to be cleaner and add more tests.
            let mut dependencies = texture_dependencies(&line_dependencies);

            // Check if anything is directly assigned to the output variable.
            // The dependent lines are sorted, so the last element is the final assignment.
            // There should be at least one assignment if the value above is some.
            let d = line_dependencies.dependent_lines.last().unwrap();
            let final_assignment = &line_dependencies.assignments[*d].input_expr;
            add_final_assignment_dependencies(final_assignment, &mut dependencies);

            dependencies
        })
        .unwrap_or_default()
}

fn add_final_assignment_dependencies(final_assignment: &Expr, dependencies: &mut Vec<SourceInput>) {
    match final_assignment {
        Expr::Variable(_) => (),
        Expr::IntConst(_) => (),
        Expr::UIntConst(_) => (),
        Expr::BoolConst(_) => (),
        Expr::FloatConst(f) => dependencies.push(SourceInput::Constant(*f)),
        Expr::DoubleConst(_) => (),
        Expr::Unary(_, _) => (),
        Expr::Binary(_, _, _) => (),
        Expr::Ternary(_, _, _) => (),
        Expr::Assignment(_, _, _) => (),
        Expr::Bracket(_, _) => (),
        Expr::FunCall(_, _) => (),
        Expr::Dot(e, channel) => {
            // TODO: Is there a cleaner way of writing this?
            if let Expr::Bracket(var, specifier) = e.as_ref() {
                if let Expr::Variable(id) = var.as_ref() {
                    if let ArraySpecifierDimension::ExplicitlySized(specifier) =
                        &specifier.dimensions.0[0]
                    {
                        if let Expr::IntConst(index) = **specifier {
                            dependencies.push(SourceInput::Buffer {
                                name: id.0.clone(),
                                index: index as usize,
                                channels: channel.0.clone(),
                            });
                        }
                    }
                }
            }
        }
        Expr::PostInc(_) => (),
        Expr::PostDec(_) => (),
        Expr::Comma(_, _) => (),
    }
}

fn texture_dependencies(dependencies: &LineDependencies) -> Vec<SourceInput> {
    dependencies
        .dependent_lines
        .iter()
        .filter_map(|d| {
            let assignment = &dependencies.assignments[*d];
            texture_identifier_name(&assignment.input_expr).map(|name| {
                // Get the initial channels used for the texture function call.
                // This defines the possible channels if we assume one access per texture.
                let mut channels = assignment.input_last_assignments[0]
                    .1
                    .as_ref()
                    .unwrap()
                    .clone();
                // If only a single channel is accessed initially, there's nothing more to do.
                if channels.len() > 1 {
                    channels = actual_channels(
                        *d,
                        &dependencies.dependent_lines,
                        &dependencies.assignments,
                        &channels,
                    );
                }

                SourceInput::Texture { name, channels }
            })
        })
        .collect()
}

fn actual_channels(
    i: usize,
    dependencies: &BTreeSet<usize>,
    assignments: &[AssignmentDependency],
    first_channels: &str,
) -> String {
    // Track which of the first accessed channels are accessed later.
    let mut has_channel = [false; 4];

    // We're given a line like "a = texture(tex, vec2(0.0)).zw;".
    // Find the next lines using the value "a".
    // This allows us to avoid tracking channels through the entire code graph.
    // TODO: Is it worth properly collecting and reducing all channel operations?
    // TODO: Is there a faster or simpler way to do this?
    for (_, second_channels) in dependencies.iter().flat_map(|d| {
        assignments[*d]
            .input_last_assignments
            .iter()
            .filter(|a| matches!(a.0, LastAssignment::LineNumber(line) if line == i))
    }) {
        if let Some(second_channels) = second_channels {
            // Get the channels accessed on lines using this texture value.
            // We'll assume that the next accesses are single channel for now.
            // Example:  b = a.y;
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

    // The second set of channels selects from the first set of channels.
    // For example, a.yz.x is accessing the first channel from yz.
    first_channels
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
                self.add_assignment(output, rh);
                Visit::Children
            }
            SimpleStatement::Declaration(glsl::syntax::Declaration::InitDeclaratorList(l)) => {
                // TODO: is it worth handling complex initializers?
                if let Some(glsl::syntax::Initializer::Simple(init)) = l.head.initializer.as_ref() {
                    let output = l.head.name.as_ref().unwrap().0.clone();
                    self.add_assignment(output, init);
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }
}

fn line_dependencies(translation_unit: &TranslationUnit, var: &str) -> Option<LineDependencies> {
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
        // Store the indices separate from the actual elements.
        // This avoids redundant clones from the visitor's dependencies.
        let mut dependent_lines = BTreeSet::new();
        dependent_lines.insert(assignment_index);

        // Follow data dependencies backwards to find all relevant lines.
        add_dependencies(&mut dependent_lines, assignment, &visitor.assignments);

        Some(LineDependencies {
            dependent_lines,
            assignments: visitor.assignments,
        })
    } else {
        // Variables not part of the code should have no dependencies.
        None
    }
}

fn add_dependencies(
    dependencies: &mut BTreeSet<usize>,
    assignment: &AssignmentDependency,
    assignments: &[AssignmentDependency],
) {
    // Recursively collect lines that the given assignment depends on.
    for (assignment, _) in &assignment.input_last_assignments {
        match assignment {
            LastAssignment::LineNumber(line) => {
                // Avoid processing the subtree rooted at a line more than once.
                if dependencies.insert(*line) {
                    let last_assignment = &assignments[*line];
                    add_dependencies(dependencies, last_assignment, assignments);
                }
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

    use glsl::{parser::Parse, syntax::ShaderStage};
    use indoc::indoc;

    fn line_dependencies_glsl(source: &str, var: &str) -> String {
        let translation_unit = ShaderStage::parse(source).unwrap();
        line_dependencies(&translation_unit, var)
            .map(|dependencies| {
                // Combine all the lines into source code again.
                // These won't exactly match the originals due to formatting differences.
                // TODO: Just store the statement in string form?
                dependencies
                    .dependent_lines
                    .into_iter()
                    .map(|d| {
                        let a = &dependencies.assignments[d];
                        format!("{} = {};", a.output, print_expr(&a.input_expr))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            })
            .unwrap_or_default()
    }

    #[test]
    fn line_dependencies_final_assignment() {
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
                a = fp_c9_data[0].x;
                b = 2.;
                c = a*b;
                d = fma(a, b, c);
                d = d+1.;
                OUT_Color.x = c+d;
            "},
            line_dependencies_glsl(glsl, "OUT_Color.x")
        );
    }

    #[test]
    fn line_dependencies_intermediate_assignment() {
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
                b = 2.;
                c = 2*b;
            "},
            line_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn line_dependencies_type_casts() {
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
                a = 0.;
                b = uint(a)>>2;
                c = data[int(b)];
            "},
            line_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn line_dependencies_missing() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 0.0;
            }
        "};

        assert_eq!("", line_dependencies_glsl(glsl, "d"));
    }

    #[test]
    fn line_dependencies_textures() {
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
                a = 1.;
                b = texture(texture1, vec2(a+2., 1.)).x;
                c = data[int(b)];
            "},
            line_dependencies_glsl(glsl, "c")
        );
    }

    #[test]
    fn input_dependencies_single_channel() {
        let glsl = indoc! {"
            void main() 
            {
                float t = 1.0;
                float a = texture(texture1, vec2(t)).xw;
                float b = a.y * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![SourceInput::Texture {
                name: "texture1".to_string(),
                channels: "w".to_string()
            }],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_single_channel_scalar() {
        let glsl = indoc! {"
            void main() 
            {
                float t = 1.0;
                float a = texture(texture1, vec2(t)).z;
                float b = a * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![SourceInput::Texture {
                name: "texture1".to_string(),
                channels: "z".to_string()
            }],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_multiple_channels() {
        let glsl = indoc! {"
            void main() 
            {
                float a = texture(texture1, vec2(1.0)).zw;
                float b = a.y + a.x;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![SourceInput::Texture {
                name: "texture1".to_string(),
                channels: "zw".to_string()
            }],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_buffers_constants_textures() {
        // Only handle parameters and constants assigned directly to outputs for now.
        // This also assumes buffers, constants, and textures are mutually exclusive.
        let glsl = indoc! {"
            void main() 
            {
                float a = texture(texture1, vec2(1.0)).x;
                out_attr1.x = a;
                out_attr1.y = fp_c4_data[1].w;
                out_attr1.z = fp_c4_data[1].y;
                out_attr1.w = 1.5;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![SourceInput::Texture {
                name: "texture1".to_string(),
                channels: "x".to_string()
            }],
            input_dependencies(&tu, "out_attr1.x")
        );
        assert_eq!(
            vec![SourceInput::Buffer {
                name: "fp_c4_data".to_string(),
                index: 1,
                channels: "w".to_string()
            }],
            input_dependencies(&tu, "out_attr1.y")
        );
        assert_eq!(
            vec![SourceInput::Buffer {
                name: "fp_c4_data".to_string(),
                index: 1,
                channels: "y".to_string()
            }],
            input_dependencies(&tu, "out_attr1.z")
        );
        assert_eq!(
            vec![SourceInput::Constant(1.5)],
            input_dependencies(&tu, "out_attr1.w")
        );
    }
}
