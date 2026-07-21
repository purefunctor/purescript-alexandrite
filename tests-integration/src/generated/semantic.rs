use std::fmt::Write;

use analyzer::QueryEngine;
use checking::core::pretty;
use checking::tree::{TermDeclarationKind, TypeDeclarationKind};
use checking::{PrettyQueries, tree};
use files::FileId;
use indexing::{TermItem, TypeItem};
use itertools::Itertools;

pub fn report(engine: &QueryEngine, id: FileId) -> String {
    let indexed = engine.indexed(id).unwrap();
    let checked = engine.checked(id).unwrap();
    let mut pretty = pretty::Pretty::new(engine, &checked);
    let mut out = String::default();

    writeln!(out, "Types").unwrap();

    for (type_id, TypeItem { name, .. }) in indexed.items.iter_types() {
        let Some(name) = name else { continue };
        let Some(declaration_id) = checked.tree.lookup_type_declaration(type_id) else {
            continue;
        };

        let declaration = &checked.tree[declaration_id];
        let (keyword, data) = match &declaration.declaration {
            TypeDeclarationKind::Data(data) => ("data", data),
            TypeDeclarationKind::Newtype(data) => ("newtype", data),
        };

        pretty.reset();
        let signature = pretty.render_signature(name, declaration.kind);
        writeln!(out, "{keyword} {signature}").unwrap();

        write_parameters(&mut out, engine, &mut pretty, data);

        let mut roles = declaration.roles.iter().map(|role| format!("{role:?}"));
        let roles = roles.join(", ");
        writeln!(out, "  roles: [{roles}]").unwrap();

        let constructors = indexed.data_constructors(type_id);
        for (&declaration_id, constructor_id) in data.constructors.iter().zip(constructors) {
            let TermItem { name, .. } = &indexed.items[constructor_id];
            let Some(name) = name else { continue };
            let declaration = &checked.tree[declaration_id];
            let TermDeclarationKind::Constructor(constructor) = &declaration.kind else {
                unreachable!("invariant violated: data declaration contains a value declaration");
            };

            pretty.reset();
            let signature = pretty.render_signature(name, declaration.type_id);
            writeln!(out, "  {signature}").unwrap();

            for &argument in constructor.arguments.iter() {
                let argument = pretty.render(argument);
                writeln!(out, "    {argument}").unwrap();
            }
        }
    }

    out
}

fn write_parameters(
    out: &mut String,
    engine: &QueryEngine,
    pretty: &mut pretty::Pretty<'_, QueryEngine>,
    data: &tree::DataDeclaration,
) {
    let mut parameters = data.parameters.iter().map(|&parameter| {
        let parameter = engine.lookup_forall_binder(parameter);
        let name = pretty.display_name(parameter.name);
        let kind = pretty.render(parameter.kind);
        format!("{name} :: {kind}")
    });
    let parameters = parameters.join(", ");
    writeln!(out, "  parameters: [{parameters}]").unwrap();
}
