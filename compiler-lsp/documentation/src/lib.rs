pub mod error;
pub mod schema;
mod warm;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use building::QueryEngine;
use checking::PrettyQueries;
use checking::core::pretty::PrettyNames;
use files::FileId;
use itertools::Itertools;
use ts_rs::{Config as TypeScriptExportConfig, TS};

pub use crate::error::Error;

#[derive(Debug)]
pub struct PackageInput {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub description: Option<String>,
    pub dependencies: BTreeMap<String, String>,
    pub modules: Vec<FileId>,
}

struct TypeEncoder<'a> {
    engine: &'a QueryEngine,
    checked: &'a checking::CheckedModule,
    package_by_file: &'a [(FileId, &'a str)],
    names: PrettyNames,
}

impl<'a> TypeEncoder<'a> {
    fn new(
        engine: &'a QueryEngine,
        checked: &'a checking::CheckedModule,
        package_by_file: &'a [(FileId, &'a str)],
    ) -> TypeEncoder<'a> {
        TypeEncoder { engine, checked, package_by_file, names: PrettyNames::new() }
    }

    fn encode_signature(&mut self, id: checking::TypeId) -> Result<schema::Type, Error> {
        self.names.reset();
        self.encode_type(id)
    }

    fn encode_type(&mut self, id: checking::TypeId) -> Result<schema::Type, Error> {
        let expression = match self.engine.lookup_type(id) {
            checking::Type::Application(function, argument) => schema::Type::Application {
                function: self.encode_boxed_type(function)?,
                argument: self.encode_boxed_type(argument)?,
            },
            checking::Type::KindApplication(function, argument) => schema::Type::KindApplication {
                function: self.encode_boxed_type(function)?,
                argument: self.encode_boxed_type(argument)?,
            },
            checking::Type::Forall(binder, body) => schema::Type::Forall {
                binder: self.encode_binder(binder)?,
                body: self.encode_boxed_type(body)?,
            },
            checking::Type::Constrained(constraint, body) => schema::Type::Constrained {
                constraint: self.encode_boxed_type(constraint)?,
                body: self.encode_boxed_type(body)?,
            },
            checking::Type::Function(argument, result) => schema::Type::Function {
                argument: self.encode_boxed_type(argument)?,
                result: self.encode_boxed_type(result)?,
            },
            checking::Type::Kinded(expression, kind) => schema::Type::Kinded {
                expression: self.encode_boxed_type(expression)?,
                kind: self.encode_boxed_type(kind)?,
            },
            checking::Type::Constructor(file_id, type_id) => schema::Type::Constructor {
                reference: self.resolve_type_reference(file_id, type_id)?,
            },
            checking::Type::Integer(value) => schema::Type::Integer { value },
            checking::Type::String(kind, value_id) => {
                let kind = match kind {
                    lowering::StringKind::String => schema::StringLiteralKind::String,
                    lowering::StringKind::RawString => schema::StringLiteralKind::RawString,
                };
                let value = self.engine.lookup_smol_str(value_id).to_string();
                schema::Type::String { kind, value }
            }
            checking::Type::Row(row_id) => {
                let row = self.engine.lookup_row_type(row_id);
                let fields = row.fields.iter().map(|field| {
                    let t = self.encode_type(field.id)?;
                    Ok(schema::TypeRowField { label: field.label.to_string(), t })
                });

                let fields = fields.collect::<Result<Vec<_>, Error>>()?;
                let tail = row.tail.map(|id| self.encode_boxed_type(id)).transpose()?;

                schema::Type::Row { fields, tail }
            }
            checking::Type::Rigid(name, _, kind) => schema::Type::Rigid {
                name: self.display_name(name),
                kind: self.encode_boxed_type(kind)?,
            },
            checking::Type::Unification(id) => schema::Type::Unification { id },
            checking::Type::Free(name_id) => {
                schema::Type::Free { name: self.engine.lookup_smol_str(name_id).to_string() }
            }
            checking::Type::Unknown(name_id) => {
                schema::Type::Unknown { name: self.engine.lookup_smol_str(name_id).to_string() }
            }
        };

        Ok(expression)
    }

    fn encode_boxed_type(&mut self, id: checking::TypeId) -> Result<Box<schema::Type>, Error> {
        Ok(Box::new(self.encode_type(id)?))
    }

    fn encode_binder(
        &mut self,
        id: checking::core::ForallBinderId,
    ) -> Result<schema::TypeBinder, Error> {
        let binder = self.engine.lookup_forall_binder(id);
        let name = self.display_name(binder.name);
        let kind = self.encode_boxed_type(binder.kind)?;

        Ok(schema::TypeBinder { name, visible: binder.visible, kind })
    }

    fn resolve_type_reference(
        &self,
        file_id: FileId,
        type_id: indexing::TypeItemId,
    ) -> Result<schema::TypeReference, Error> {
        let package = self.package_by_file.iter().find_map(|&(id, package)| {
            if id == file_id { Some(package.to_string()) } else { None }
        });

        let (parsed, _) = self.engine.parsed(file_id)?;
        let module = parsed.module_name().map(|name| name.to_string());

        let indexed = self.engine.indexed(file_id)?;
        let name = indexed.items[type_id].name.as_ref().map(|name| name.to_string());

        Ok(schema::TypeReference { package, module, name })
    }

    fn display_name(&mut self, name: checking::core::Name) -> String {
        self.names.display_name(self.engine, &self.checked.names, name).to_string()
    }
}

pub fn export_typescript(output: PathBuf) -> Result<(), Error> {
    let typescript = TypeScriptExportConfig::new().with_out_dir(output);

    schema::Package::export_all(&typescript)?;
    schema::Module::export_all(&typescript)?;

    Ok(())
}

pub fn warm_queries(engine: &QueryEngine, modules: &[FileId]) -> Result<(), Error> {
    warm::warm_documentation_queries(engine, modules)
}

pub fn render_package_manifest(
    engine: &QueryEngine,
    package: &PackageInput,
) -> Result<schema::Package, Error> {
    let modules = package.modules.iter().map(|&id| {
        let (parsed, _) = engine.parsed(id)?;
        Ok(parsed.module_name().map(|name| name.to_string()))
    });

    let modules = modules.collect::<Result<Vec<_>, Error>>()?;

    Ok(schema::Package {
        name: String::clone(&package.name),
        version: String::clone(&package.version),
        license: package.license.clone(),
        description: package.description.clone(),
        dependencies: BTreeMap::clone(&package.dependencies),
        modules: modules.into_iter().flatten().collect_vec(),
    })
}

pub fn render_module(
    engine: &QueryEngine,
    file_id: FileId,
    package_by_file: &[(FileId, &str)],
) -> Result<Option<schema::Module>, Error> {
    let (parsed, _) = engine.parsed(file_id)?;
    let indexed = engine.indexed(file_id)?;
    let lowered = engine.lowered(file_id)?;
    let checked = engine.checked(file_id)?;
    let documented = engine.documented(file_id)?;

    let Some(name) = parsed.module_name().map(|name| name.to_string()) else {
        return Ok(None);
    };

    let mut terms = vec![];
    let mut types = vec![];
    let mut type_encoder = TypeEncoder::new(engine, &checked, package_by_file);
    let (mut instances_by_parent, mut nested_terms) =
        instance_parent_map(file_id, &indexed, &lowered, &checked);

    for (_, type_item) in indexed.items.iter_types() {
        match &type_item.kind {
            indexing::TypeItemKind::Data { constructors, .. }
            | indexing::TypeItemKind::Newtype { constructors, .. } => {
                nested_terms.extend(constructors.iter().copied());
            }
            indexing::TypeItemKind::Class { members, .. } => {
                nested_terms.extend(members.iter().copied());
            }
            indexing::TypeItemKind::Synonym { .. }
            | indexing::TypeItemKind::Foreign { .. }
            | indexing::TypeItemKind::Operator { .. } => {}
        }
    }

    for (term_id, term_item) in indexed.items.iter_terms() {
        if nested_terms.contains(&term_id) {
            continue;
        }

        terms.push(encode_term_item(term_id, term_item, &documented, &checked, &mut type_encoder)?);
    }

    for (type_id, type_item) in indexed.items.iter_types() {
        let type_documentation = documented.types.get(&type_id);

        let name = type_item.name.as_ref().map(|name| name.to_string());
        let documentation = type_documentation.map(|t| t.documentation.to_string());
        let signature = checked
            .lookup_type(type_id)
            .map(|signature| type_encoder.encode_signature(signature))
            .transpose()?;

        let kind = match &type_item.kind {
            indexing::TypeItemKind::Data { .. } => schema::TypeKind::Data,
            indexing::TypeItemKind::Newtype { .. } => schema::TypeKind::Newtype,
            indexing::TypeItemKind::Synonym { .. } => schema::TypeKind::Synonym,
            indexing::TypeItemKind::Class { .. } => schema::TypeKind::Class,
            indexing::TypeItemKind::Foreign { .. } => schema::TypeKind::Foreign,
            indexing::TypeItemKind::Operator { .. } => schema::TypeKind::Operator,
        };

        let constructors = match &type_item.kind {
            indexing::TypeItemKind::Data { constructors, .. }
            | indexing::TypeItemKind::Newtype { constructors, .. } => encode_term_items(
                &mut type_encoder,
                &indexed,
                &documented,
                &checked,
                constructors.iter().copied(),
            )?,
            indexing::TypeItemKind::Synonym { .. }
            | indexing::TypeItemKind::Class { .. }
            | indexing::TypeItemKind::Foreign { .. }
            | indexing::TypeItemKind::Operator { .. } => vec![],
        };

        let members = match &type_item.kind {
            indexing::TypeItemKind::Class { members, .. } => encode_term_items(
                &mut type_encoder,
                &indexed,
                &documented,
                &checked,
                members.iter().copied(),
            )?,
            indexing::TypeItemKind::Data { .. }
            | indexing::TypeItemKind::Newtype { .. }
            | indexing::TypeItemKind::Synonym { .. }
            | indexing::TypeItemKind::Foreign { .. }
            | indexing::TypeItemKind::Operator { .. } => vec![],
        };

        let instances = instances_by_parent.remove(&type_id).unwrap_or_default();
        let instances =
            encode_term_items(&mut type_encoder, &indexed, &documented, &checked, instances)?;

        let expands_to = checked
            .lookup_synonym(type_id)
            .map(|synonym| type_encoder.encode_signature(synonym.synonym))
            .transpose()?;

        types.push(schema::TypeItem {
            name,
            documentation,
            signature,
            kind,
            constructors,
            members,
            instances,
            expands_to,
        });
    }

    let documentation = Some(documented.documentation.to_string());

    Ok(Some(schema::Module { name, documentation, terms, types }))
}

fn encode_term_items(
    type_encoder: &mut TypeEncoder<'_>,
    indexed: &indexing::IndexedModule,
    documented: &documenting::DocumentedModule,
    checked: &checking::CheckedModule,
    terms: impl IntoIterator<Item = indexing::TermItemId>,
) -> Result<Vec<schema::TermItem>, Error> {
    let terms = terms.into_iter().map(|term_id| {
        let term_item = &indexed.items[term_id];
        encode_term_item(term_id, term_item, documented, checked, type_encoder)
    });
    terms.collect()
}

fn encode_term_item(
    term_id: indexing::TermItemId,
    term_item: &indexing::TermItem,
    documented: &documenting::DocumentedModule,
    checked: &checking::CheckedModule,
    type_encoder: &mut TypeEncoder<'_>,
) -> Result<schema::TermItem, Error> {
    let term_documentation = documented.terms.get(&term_id);

    let name = term_item.name.as_ref().map(|name| name.to_string());
    let documentation = term_documentation.map(|term| term.documentation.to_string());
    let signature = term_signature(term_id, term_item, checked)
        .map(|signature| type_encoder.encode_signature(signature))
        .transpose()?;
    let kind = encode_term_kind(&term_item.kind);

    Ok(schema::TermItem { name, documentation, signature, kind })
}

fn term_signature(
    term_id: indexing::TermItemId,
    term_item: &indexing::TermItem,
    checked: &checking::CheckedModule,
) -> Option<checking::TypeId> {
    match &term_item.kind {
        indexing::TermItemKind::Instance { id } => {
            checked.lookup_instance(*id).map(|instance| instance.signature)
        }
        indexing::TermItemKind::Derive { id } => {
            checked.lookup_derived(*id).map(|instance| instance.signature)
        }
        indexing::TermItemKind::ClassMember { .. }
        | indexing::TermItemKind::Constructor { .. }
        | indexing::TermItemKind::Foreign { .. }
        | indexing::TermItemKind::Operator { .. }
        | indexing::TermItemKind::Value { .. } => checked.lookup_term(term_id),
    }
}

fn encode_term_kind(kind: &indexing::TermItemKind) -> schema::TermKind {
    match kind {
        indexing::TermItemKind::ClassMember { .. } => schema::TermKind::ClassMember,
        indexing::TermItemKind::Constructor { .. } => schema::TermKind::Constructor,
        indexing::TermItemKind::Derive { .. } => schema::TermKind::Derive,
        indexing::TermItemKind::Foreign { .. } => schema::TermKind::Foreign,
        indexing::TermItemKind::Instance { .. } => schema::TermKind::Instance,
        indexing::TermItemKind::Operator { .. } => schema::TermKind::Operator,
        indexing::TermItemKind::Value { .. } => schema::TermKind::Value,
    }
}

fn instance_parent_map(
    file_id: FileId,
    indexed: &indexing::IndexedModule,
    lowered: &lowering::LoweredModule,
    checked: &checking::CheckedModule,
) -> (BTreeMap<indexing::TypeItemId, Vec<indexing::TermItemId>>, BTreeSet<indexing::TermItemId>) {
    let mut instances_by_parent =
        BTreeMap::<indexing::TypeItemId, Vec<indexing::TermItemId>>::new();
    let mut nested_terms = BTreeSet::new();

    for (term_id, term_item) in indexed.items.iter_terms() {
        if !matches!(
            term_item.kind,
            indexing::TermItemKind::Instance { .. } | indexing::TermItemKind::Derive { .. }
        ) {
            continue;
        }

        let parents = instance_parents(file_id, indexed, lowered, checked, term_id, term_item);
        if parents.is_empty() {
            continue;
        }

        nested_terms.insert(term_id);
        for parent in parents {
            instances_by_parent.entry(parent).or_default().push(term_id);
        }
    }

    (instances_by_parent, nested_terms)
}

fn instance_parents(
    file_id: FileId,
    indexed: &indexing::IndexedModule,
    lowered: &lowering::LoweredModule,
    checked: &checking::CheckedModule,
    term_id: indexing::TermItemId,
    term_item: &indexing::TermItem,
) -> BTreeSet<indexing::TypeItemId> {
    let mut parents = BTreeSet::new();

    let checked_instance = match &term_item.kind {
        indexing::TermItemKind::Instance { id } => checked.lookup_instance(*id),
        indexing::TermItemKind::Derive { id } => checked.lookup_derived(*id),
        indexing::TermItemKind::ClassMember { .. }
        | indexing::TermItemKind::Constructor { .. }
        | indexing::TermItemKind::Foreign { .. }
        | indexing::TermItemKind::Operator { .. }
        | indexing::TermItemKind::Value { .. } => None,
    };

    if let Some(instance) = checked_instance {
        let (parent_file, parent_type) = instance.resolution;
        if parent_file == file_id {
            parents.insert(parent_type);
        }
    }

    let Some(term_item) = lowered.info.get_term_item(term_id) else { return parents };
    let arguments = match term_item {
        lowering::TermItemIr::Instance { arguments, .. }
        | lowering::TermItemIr::Derive { arguments, .. } => arguments,
        lowering::TermItemIr::ClassMember { .. }
        | lowering::TermItemIr::Constructor { .. }
        | lowering::TermItemIr::Foreign { .. }
        | lowering::TermItemIr::Operator { .. }
        | lowering::TermItemIr::ValueGroup { .. } => return parents,
    };

    for &argument in arguments.iter() {
        collect_instance_type_parents(file_id, indexed, &lowered.info, argument, &mut parents);
    }

    parents
}

fn collect_instance_type_parents(
    file_id: FileId,
    indexed: &indexing::IndexedModule,
    info: &lowering::LoweringInfo,
    type_id: lowering::TypeId,
    parents: &mut BTreeSet<indexing::TypeItemId>,
) {
    let Some(kind) = info.get_type_kind(type_id) else { return };

    match kind {
        lowering::TypeKind::Constructor { resolution } => {
            if let Some((parent_file, parent_type)) = resolution
                && *parent_file == file_id
                && instance_type_parent(indexed, *parent_type)
            {
                parents.insert(*parent_type);
            }
        }
        lowering::TypeKind::ApplicationChain { function, arguments } => {
            if let Some(function) = function {
                collect_instance_type_parents(file_id, indexed, info, *function, parents);
            }
            for &argument in arguments.iter() {
                collect_instance_type_parents(file_id, indexed, info, argument, parents);
            }
        }
        lowering::TypeKind::Arrow { argument, result } => {
            if let Some(argument) = argument {
                collect_instance_type_parents(file_id, indexed, info, *argument, parents);
            }
            if let Some(result) = result {
                collect_instance_type_parents(file_id, indexed, info, *result, parents);
            }
        }
        lowering::TypeKind::Constrained { constraint, constrained } => {
            if let Some(constraint) = constraint {
                collect_instance_type_parents(file_id, indexed, info, *constraint, parents);
            }
            if let Some(constrained) = constrained {
                collect_instance_type_parents(file_id, indexed, info, *constrained, parents);
            }
        }
        lowering::TypeKind::Forall { inner, .. } => {
            if let Some(inner) = inner {
                collect_instance_type_parents(file_id, indexed, info, *inner, parents);
            }
        }
        lowering::TypeKind::Kinded { type_, kind } => {
            if let Some(type_) = type_ {
                collect_instance_type_parents(file_id, indexed, info, *type_, parents);
            }
            if let Some(kind) = kind {
                collect_instance_type_parents(file_id, indexed, info, *kind, parents);
            }
        }
        lowering::TypeKind::OperatorChain { head, tail } => {
            if let Some(head) = head {
                collect_instance_type_parents(file_id, indexed, info, *head, parents);
            }
            for pair in tail.iter() {
                if let Some(element) = pair.element {
                    collect_instance_type_parents(file_id, indexed, info, element, parents);
                }
            }
        }
        lowering::TypeKind::Record { items, tail } | lowering::TypeKind::Row { items, tail } => {
            for item in items.iter() {
                if let Some(type_) = item.type_ {
                    collect_instance_type_parents(file_id, indexed, info, type_, parents);
                }
            }
            if let Some(tail) = tail {
                collect_instance_type_parents(file_id, indexed, info, *tail, parents);
            }
        }
        lowering::TypeKind::Parenthesized { parenthesized } => {
            if let Some(parenthesized) = parenthesized {
                collect_instance_type_parents(file_id, indexed, info, *parenthesized, parents);
            }
        }
        lowering::TypeKind::Operator { .. }
        | lowering::TypeKind::Hole
        | lowering::TypeKind::Integer { .. }
        | lowering::TypeKind::String { .. }
        | lowering::TypeKind::Variable { .. }
        | lowering::TypeKind::Wildcard => {}
    }
}

fn instance_type_parent(indexed: &indexing::IndexedModule, type_id: indexing::TypeItemId) -> bool {
    matches!(
        indexed.items[type_id].kind,
        indexing::TypeItemKind::Data { .. }
            | indexing::TypeItemKind::Newtype { .. }
            | indexing::TypeItemKind::Synonym { .. }
            | indexing::TypeItemKind::Foreign { .. }
    )
}
