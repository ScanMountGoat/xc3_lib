// TODO: make dependencies and annotation into a library?
use std::collections::{BTreeSet, HashMap};

use glsl_lang::{
    ast::{
        DeclarationData, Expr, ExprData, FunIdentifierData, Identifier, InitializerData, Statement,
        StatementData, TranslationUnit,
    },
    parse::DefaultParse,
    transpiler::glsl::{show_expr, FormattingState},
    visitor::{Host, Visit, Visitor},
};
use xc3_model::shader_database::{BufferDependency, Dependency, TexCoord, TextureDependency};

use crate::annotation::shader_source_no_extensions;

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
            output_var: output,
            input_last_assignments,
            assignment_input: input.clone(),
        };
        // The visitor doesn't track line numbers.
        // We only need to look up the assignments, so use the index instead.
        self.last_assignment_index
            .insert(assignment.output_var.clone(), self.assignments.len());
        self.assignments.push(assignment);
    }
}

impl Visitor for AssignmentVisitor {
    fn visit_statement(&mut self, statement: &Statement) -> Visit {
        match &statement.content {
            StatementData::Expression(expr) => {
                if let Some(ExprData::Assignment(lh, _, rh)) =
                    expr.content.0.as_ref().map(|c| &c.content)
                {
                    let output = print_expr(lh);
                    self.add_assignment(output, rh);
                }

                Visit::Children
            }
            StatementData::Declaration(decl) => {
                if let DeclarationData::InitDeclaratorList(l) = &decl.content {
                    // TODO: is it worth handling complex initializers?
                    if let Some(InitializerData::Simple(init)) =
                        l.head.initializer.as_ref().map(|c| &c.content)
                    {
                        let output = l.head.name.as_ref().unwrap().0.clone();
                        self.add_assignment(output.to_string(), init);
                    }
                }

                Visit::Children
            }
            _ => Visit::Children,
        }
    }
}

#[derive(Debug, Clone)]
struct AssignmentDependency {
    output_var: String,

    assignment_input: Expr,

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
    dependent_assignment_indices: BTreeSet<usize>,
    assignments: Vec<AssignmentDependency>,
}

pub fn input_dependencies(translation_unit: &TranslationUnit, var: &str) -> Vec<Dependency> {
    line_dependencies(translation_unit, var)
        .map(|line_dependencies| {
            // TODO: Rework this later to make fewer assumptions about the code structure.
            // TODO: Rework this to be cleaner and add more tests.
            let mut dependencies = texture_dependencies(&line_dependencies);

            // Check if anything is directly assigned to the output variable.
            // The dependent lines are sorted, so the last element is the final assignment.
            // There should be at least one assignment if the value above is some.
            let d = line_dependencies
                .dependent_assignment_indices
                .last()
                .unwrap();
            let final_assignment = &line_dependencies.assignments[*d].assignment_input;
            add_assignment_dependencies(final_assignment, &mut dependencies);

            dependencies
        })
        .unwrap_or_default()
}

fn add_assignment_dependencies(expr: &Expr, dependencies: &mut Vec<Dependency>) {
    match &expr.content {
        ExprData::Variable(_) => (),
        ExprData::IntConst(_) => (),
        ExprData::UIntConst(_) => (),
        ExprData::BoolConst(_) => (),
        ExprData::FloatConst(f) => dependencies.push(Dependency::Constant((*f).into())),
        ExprData::DoubleConst(_) => (),
        ExprData::Unary(_, _) => (),
        ExprData::Binary(_, _, _) => (),
        ExprData::Ternary(_, _, _) => (),
        ExprData::Assignment(_, _, _) => (),
        ExprData::Bracket(_, _) => (),
        ExprData::FunCall(_, _) => (),
        ExprData::Dot(e, channel) => {
            if let Some(buffer) = buffer_dependency_from_dot_expr(e, channel) {
                dependencies.push(Dependency::Buffer(buffer));
            }
        }
        ExprData::PostInc(_) => (),
        ExprData::PostDec(_) => (),
        ExprData::Comma(_, _) => (),
    }
}

fn buffer_dependency_from_dot_expr(e: &Expr, channel: &Identifier) -> Option<BufferDependency> {
    // TODO: Is there a cleaner way of writing this?
    if let ExprData::Bracket(var, specifier) = &e.content {
        if let ExprData::IntConst(index) = &specifier.content {
            match &var.as_ref().content {
                ExprData::Variable(id) => {
                    // buffer[index].x
                    return Some(BufferDependency {
                        name: id.content.to_string(),
                        field: String::new(), // TODO: use none instead?
                        index: *index as usize,
                        channels: channel.content.to_string(),
                    });
                }
                ExprData::Dot(e, field) => {
                    if let ExprData::Variable(id) = &e.content {
                        // buffer.field[index].x
                        return Some(BufferDependency {
                            name: id.content.to_string(),
                            field: field.0.to_string(),
                            index: *index as usize,
                            channels: channel.content.to_string(),
                        });
                    }
                }
                _ => (),
            }
        }
    }

    None
}

fn texture_dependencies(dependencies: &LineDependencies) -> Vec<Dependency> {
    dependencies
        .dependent_assignment_indices
        .iter()
        .filter_map(|d| {
            let assignment = &dependencies.assignments[*d];
            texture_identifier_name(&assignment.assignment_input).and_then(|name| {
                let texcoord = texcoord_name_channels(assignment, dependencies);

                // Get the initial channels used for the texture function call.
                // This defines the possible channels if we assume one access per texture.
                // TODO: Why is this sometimes none?
                let mut channels = assignment.input_last_assignments[0].1.as_ref()?.clone();

                // If only a single channel is accessed initially, there's nothing more to do.
                if channels.len() > 1 {
                    channels = actual_channels(
                        *d,
                        &dependencies.dependent_assignment_indices,
                        &dependencies.assignments,
                        &channels,
                    );
                }

                // TODO: Just detect if texmat is part of the globals in input_last_assignment?
                // We can assume these are in the order texture, uv.x, uv.y, etc.
                // This ensures we map the correct parameter to the correct coordinate.
                let params: Vec<_> = assignment
                    .input_last_assignments
                    .iter()
                    .skip(1)
                    .filter_map(|(a, _)| {
                        // TODO: Should this be recursive?
                        if let LastAssignment::LineNumber(l) = a {
                            find_buffer_parameter(&dependencies.assignments[*l].assignment_input)
                        } else {
                            None
                        }
                    })
                    .collect();
                // dbg!(&assignment.input_last_assignments);

                let texcoord = texcoord.map(|(name, channels)| TexCoord {
                    name,
                    channels,
                    params,
                });

                // TODO: Add Vec<BufferDependency> for the texcoord?
                // TODO: Store Vec<BufferDependency> for both vert and frag?
                Some(Dependency::Texture(TextureDependency {
                    name,
                    texcoord,
                    channels,
                }))
            })
        })
        .collect()
}

fn find_buffer_parameter(expr: &Expr) -> Option<BufferDependency> {
    // TODO: Share code with add_vars using some sort of visitor?
    // TODO: Create a visitor for dot expressions?
    // TODO: Handle any binary op?
    match &expr.content {
        ExprData::Variable(_) => None,
        ExprData::IntConst(_) => None,
        ExprData::UIntConst(_) => None,
        ExprData::BoolConst(_) => None,
        ExprData::FloatConst(_) => None,
        ExprData::DoubleConst(_) => None,
        ExprData::Unary(_, _) => None,
        ExprData::Binary(_, e1, e2) => {
            find_buffer_parameter(e1).or_else(|| find_buffer_parameter(e2))
        }
        ExprData::Ternary(_, _, _) => None,
        ExprData::Assignment(_, _, _) => None,
        ExprData::Bracket(_, _) => None,
        ExprData::FunCall(_, _) => None,
        ExprData::Dot(e, channel) => buffer_dependency_from_dot_expr(e, channel),
        ExprData::PostInc(_) => None,
        ExprData::PostDec(_) => None,
        ExprData::Comma(_, _) => None,
    }
}

fn texcoord_name_channels(
    assignment: &AssignmentDependency,
    dependencies: &LineDependencies,
) -> Option<(String, String)> {
    // Search dependent lines to find what UV attribute is used like in_attr3.zw.
    // Skip the texture name in the first function argument.
    // TODO: Find a better way to combine UV channels.
    let (u_name, u_channels) =
        find_uv_attribute_channel(assignment.input_last_assignments.get(1..2)?, dependencies)?;
    let (_, v_channels) =
        find_uv_attribute_channel(assignment.input_last_assignments.get(2..3)?, dependencies)?;
    Some((u_name, u_channels + &v_channels))
}

fn find_uv_attribute_channel(
    last_assignments: &[(LastAssignment, Option<String>)],
    dependencies: &LineDependencies,
) -> Option<(String, String)> {
    // Recurse backwards from the current assignment until we find a global variable.
    // A global variable should have no other assignments to avoid detecting parameters.
    match last_assignments {
        [(LastAssignment::Global(g), c)] => Some((g.clone(), c.clone()?)),
        assignments => assignments.iter().find_map(|(a, _)| match a {
            LastAssignment::LineNumber(i) => find_uv_attribute_channel(
                &dependencies.assignments[*i].input_last_assignments,
                dependencies,
            ),
            LastAssignment::Global(_) => None,
        }),
    }
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

fn texture_identifier_name(expr: &Expr) -> Option<String> {
    // Assume textures are only accessed in statements with a single texture function.
    // Accesses may have channels like "texture(the_tex, vec2(0.5)).rgb".
    match &expr.content {
        ExprData::FunCall(id, es) => {
            if let FunIdentifierData::Expr(expr) = &id.content {
                if let ExprData::Variable(id) = &expr.content {
                    if id.content.0.as_str().contains("texture") {
                        // Get the texA from "texture(texA, ...)".
                        if let ExprData::Variable(id) = &es[0].content {
                            return Some(id.content.0.to_string());
                        }
                    }
                }
            }
        }
        ExprData::Dot(e, _) => return texture_identifier_name(e),
        _ => (),
    }

    None
}

// TODO: Can this be written as a visitor for variables and dot?
fn add_vars(
    expr: &Expr,
    vars: &mut Vec<(LastAssignment, Option<String>)>,
    most_recent_assignment: &HashMap<String, usize>,
    channel: Option<&str>,
) {
    // Collect any variables used in an expression.
    // Code like fma(a, b, c) should return [a, b, c].
    // TODO: Include constants?
    match &expr.content {
        ExprData::Variable(i) => {
            // The base case is a single variable like temp_01.
            // Also handle values like buffer or texture names.
            let assignment = match most_recent_assignment.get(i.content.0.as_str()) {
                Some(i) => LastAssignment::LineNumber(*i),
                None => LastAssignment::Global(i.content.0.to_string()),
            };
            vars.push((assignment, channel.map(|c| c.to_string())));
        }
        ExprData::IntConst(_) => (),
        ExprData::UIntConst(_) => (),
        ExprData::BoolConst(_) => (),
        ExprData::FloatConst(_) => (),
        ExprData::DoubleConst(_) => (),
        ExprData::Unary(_, e) => add_vars(e, vars, most_recent_assignment, channel),
        ExprData::Binary(_, lh, rh) => {
            add_vars(lh, vars, most_recent_assignment, channel);
            add_vars(rh, vars, most_recent_assignment, channel);
        }
        ExprData::Ternary(a, b, c) => {
            add_vars(a, vars, most_recent_assignment, channel);
            add_vars(b, vars, most_recent_assignment, channel);
            add_vars(c, vars, most_recent_assignment, channel);
        }
        ExprData::Assignment(_, _, _) => todo!(),
        ExprData::Bracket(e, specifier) => {
            // Expressions like array[index] depend on index.
            // TODO: Do we also need to depend on array itself?
            add_vars(e, vars, most_recent_assignment, channel);
            add_vars(specifier, vars, most_recent_assignment, channel);
        }
        ExprData::FunCall(_, es) => {
            for e in es {
                add_vars(e, vars, most_recent_assignment, channel);
            }
        }
        ExprData::Dot(e, channel) => {
            // Track the channels accessed by expressions like "value.rgb".
            add_vars(
                e,
                vars,
                most_recent_assignment,
                Some(channel.content.0.as_str()),
            )
        }
        ExprData::PostInc(e) => add_vars(e, vars, most_recent_assignment, channel),
        ExprData::PostDec(e) => add_vars(e, vars, most_recent_assignment, channel),
        ExprData::Comma(_, _) => todo!(),
    }
}

fn print_expr(expr: &Expr) -> String {
    let mut text = String::new();
    show_expr(&mut text, expr, &mut FormattingState::default()).unwrap();
    text
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
        .rfind(|(_, a)| a.output_var == var)
    {
        // Store the indices separate from the actual elements.
        // This avoids redundant clones from the visitor's dependencies.
        let mut dependent_lines = BTreeSet::new();
        dependent_lines.insert(assignment_index);

        // Follow data dependencies backwards to find all relevant lines.
        add_dependencies(&mut dependent_lines, assignment, &visitor.assignments);

        Some(LineDependencies {
            dependent_assignment_indices: dependent_lines,
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

pub fn glsl_dependencies(source: &str, var: &str) -> String {
    // TODO: Correctly handle if statements?
    let source = shader_source_no_extensions(source);
    let translation_unit = TranslationUnit::parse(source).unwrap();
    line_dependencies(&translation_unit, var)
        .map(|dependencies| {
            // Combine all the lines into source code again.
            // These won't exactly match the originals due to formatting differences.
            dependencies
                .dependent_assignment_indices
                .into_iter()
                .map(|d| {
                    let a = &dependencies.assignments[d];
                    format!("{} = {};", a.output_var, print_expr(&a.assignment_input))
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        })
        .unwrap_or_default()
}

// TODO: should this be recursive?
pub fn find_buffer_parameters(
    translation_unit: &TranslationUnit,
    var: &str,
) -> Vec<BufferDependency> {
    line_dependencies(translation_unit, var)
        .map(|dependencies| {
            let assignment_index = dependencies.dependent_assignment_indices.last().unwrap();
            let assignment = &dependencies.assignments[*assignment_index];
            assignment
                .input_last_assignments
                .iter()
                .filter_map(|(a, _)| {
                    if let LastAssignment::LineNumber(l) = a {
                        find_buffer_parameter(&dependencies.assignments[*l].assignment_input)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use glsl_lang::ast::TranslationUnit;
    use indoc::indoc;

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
                c = a * b;
                d = fma(a, b, c);
                d = d + 1.;
                OUT_Color.x = c + d;
            "},
            glsl_dependencies(glsl, "OUT_Color.x")
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
                c = 2 * b;
            "},
            glsl_dependencies(glsl, "c")
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
                b = uint(a) >> 2;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c")
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

        assert_eq!("", glsl_dependencies(glsl, "d"));
    }

    #[test]
    fn line_dependencies_textures() {
        let glsl = indoc! {"
            void main() 
            {
                float a = 1.0;
                float a2 = a * 5.0;
                float b = texture(texture1, vec2(a2 + 2.0, 1.0)).x;
                float c = data[int(b)];
            }
        "};

        assert_eq!(
            indoc! {"
                a = 1.;
                a2 = a * 5.;
                b = texture(texture1, vec2(a2 + 2., 1.)).x;
                c = data[int(b)];
            "},
            glsl_dependencies(glsl, "c")
        );
    }

    #[test]
    fn input_dependencies_single_channel() {
        let glsl = indoc! {"
            void main() 
            {
                float x = in_attr0.x;
                float y = in_attr0.w;
                float x2 = x;
                float y2 = y;
                float a = texture(texture1, vec2(x2, y2)).xw;
                float b = a.y * 2.0;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "w".to_string(),
                texcoord: Some(TexCoord {
                    name: "in_attr0".to_string(),
                    channels: "xw".to_string(),
                    params: Vec::new()
                })
            })],
            input_dependencies(&tu, "b")
        );
    }

    #[test]
    fn input_dependencies_scale_tex_matrix() {
        let glsl = indoc! {"
            void main() 
            {
                temp_0 = in_attr4.x;
                temp_1 = in_attr4.y;
                temp_141 = temp_0 * U_Mate.gTexMat[0].x;
                temp_147 = temp_0 * U_Mate.gTexMat[1].x;
                temp_148 = fma(temp_1, U_Mate.gTexMat[0].y, temp_141);
                temp_151 = fma(temp_1, U_Mate.gTexMat[1].y, temp_147);
                temp_152 = fma(0., U_Mate.gTexMat[1].z, temp_151);
                temp_154 = fma(0., U_Mate.gTexMat[0].z, temp_148);
                temp_155 = temp_152 + U_Mate.gTexMat[1].w;
                temp_160 = temp_154 + U_Mate.gTexMat[0].w;
                temp_162 = texture(gTResidentTex05, vec2(temp_160, temp_155)).wyz;
                temp_163 = temp_162.x; 
            }
        "};

        // TODO: reliably detect channels even with matrix multiplication?
        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex05".to_string(),
                channels: "w".to_string(),
                texcoord: Some(TexCoord {
                    name: "in_attr4".to_string(),
                    channels: "yy".to_string(),
                    params: vec![
                        BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gTexMat".to_string(),
                            index: 0,
                            channels: "w".to_string()
                        },
                        BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gTexMat".to_string(),
                            index: 1,
                            channels: "w".to_string()
                        }
                    ]
                })
            })],
            input_dependencies(&tu, "temp_163")
        );
    }

    #[test]
    fn input_dependencies_scale_parameter() {
        let glsl = indoc! {"
            void main() 
            {
                temp_0 = in_attr4.x;
                temp_1 = in_attr4.y;
                test = 0.5;
                temp_121 = temp_1 * U_Mate.gWrkFl4[0].w;
                temp_157 = temp_0 * U_Mate.gWrkFl4[0].z;
                temp_169 = texture(gTResidentTex04, vec2(temp_157, temp_121)).xyz;
                temp_170 = temp_169.x; 
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "gTResidentTex04".to_string(),
                channels: "x".to_string(),
                texcoord: Some(TexCoord {
                    name: "in_attr4".to_string(),
                    channels: "xy".to_string(),
                    params: vec![
                        BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 0,
                            channels: "z".to_string()
                        },
                        BufferDependency {
                            name: "U_Mate".to_string(),
                            field: "gWrkFl4".to_string(),
                            index: 0,
                            channels: "w".to_string()
                        }
                    ]
                })
            })],
            input_dependencies(&tu, "temp_170")
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
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "z".to_string(),
                texcoord: None
            })],
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
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "zw".to_string(),
                texcoord: None
            })],
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
                out_attr1.y = U_Mate.data[1].w;
                out_attr1.z = uniform_data[3].y;
                out_attr1.w = 1.5;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert_eq!(
            vec![Dependency::Texture(TextureDependency {
                name: "texture1".to_string(),
                channels: "x".to_string(),
                texcoord: None
            })],
            input_dependencies(&tu, "out_attr1.x")
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "U_Mate".to_string(),
                field: "data".to_string(),
                index: 1,
                channels: "w".to_string()
            })],
            input_dependencies(&tu, "out_attr1.y")
        );
        assert_eq!(
            vec![Dependency::Buffer(BufferDependency {
                name: "uniform_data".to_string(),
                field: String::new(),
                index: 3,
                channels: "y".to_string()
            })],
            input_dependencies(&tu, "out_attr1.z")
        );
        assert_eq!(
            vec![Dependency::Constant(1.5.into())],
            input_dependencies(&tu, "out_attr1.w")
        );
    }

    #[test]
    fn find_vertex_texcoord_parameters() {
        let glsl = indoc! {"
            void main() {
                temp_62 = vTex0.x;
                temp_64 = vTex0.y;
                temp_119 = temp_62 * U_Mate.gWrkFl4[0].x;
                out_attr4.z = temp_119;
                out_attr4.x = temp_62;
                out_attr4.y = temp_64;
                temp_179 = temp_64 * U_Mate.gWrkFl4[0].y;
                out_attr4.w = temp_179;
            }
        "};

        let tu = TranslationUnit::parse(glsl).unwrap();
        assert!(find_buffer_parameters(&tu, "out_attr4.x").is_empty());
        assert!(find_buffer_parameters(&tu, "out_attr4.y").is_empty());
        assert_eq!(
            vec![BufferDependency {
                name: "U_Mate".to_string(),
                field: "gWrkFl4".to_string(),
                index: 0,
                channels: "x".to_string()
            }],
            find_buffer_parameters(&tu, "out_attr4.z")
        );
        assert_eq!(
            vec![BufferDependency {
                name: "U_Mate".to_string(),
                field: "gWrkFl4".to_string(),
                index: 0,
                channels: "y".to_string()
            }],
            find_buffer_parameters(&tu, "out_attr4.w")
        );
    }
}
