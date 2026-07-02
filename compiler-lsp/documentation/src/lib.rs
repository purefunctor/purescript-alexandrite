pub mod error;
pub mod schema;
mod warm;

use std::collections::BTreeMap;
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
    let checked = engine.checked(file_id)?;
    let documented = engine.documented(file_id)?;

    let Some(name) = parsed.module_name().map(|name| name.to_string()) else {
        return Ok(None);
    };

    let mut terms = vec![];
    let mut types = vec![];
    let mut type_encoder = TypeEncoder::new(engine, &checked, package_by_file);

    for (term_id, term_item) in indexed.items.iter_terms() {
        let term_documentation = documented.terms.get(&term_id);

        let name = term_item.name.as_ref().map(|name| name.to_string());
        let documentation = term_documentation.map(|t| t.documentation.to_string());
        let signature = checked
            .lookup_term(term_id)
            .map(|signature| type_encoder.encode_signature(signature))
            .transpose()?;

        let kind = match &term_item.kind {
            indexing::TermItemKind::ClassMember { .. } => schema::TermKind::ClassMember,
            indexing::TermItemKind::Constructor { .. } => schema::TermKind::Constructor,
            indexing::TermItemKind::Derive { .. } => schema::TermKind::Derive,
            indexing::TermItemKind::Foreign { .. } => schema::TermKind::Foreign,
            indexing::TermItemKind::Instance { .. } => schema::TermKind::Instance,
            indexing::TermItemKind::Operator { .. } => schema::TermKind::Operator,
            indexing::TermItemKind::Value { .. } => schema::TermKind::Value,
        };

        terms.push(schema::TermItem { name, documentation, signature, kind });
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

        types.push(schema::TypeItem { name, documentation, signature, kind });
    }

    let documentation = Some(documented.documentation.to_string());

    Ok(Some(schema::Module { name, documentation, terms, types }))
}
