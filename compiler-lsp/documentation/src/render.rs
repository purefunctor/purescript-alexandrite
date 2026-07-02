use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use building::QueryEngine;
use checking::PrettyQueries;
use checking::core::pretty::PrettyNames;
use files::FileId;

use crate::{Error, schema};

#[derive(Debug)]
pub struct PackageInput<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub license: Option<&'a str>,
    pub description: Option<&'a str>,
    pub dependencies: &'a BTreeMap<String, String>,
    pub modules: &'a [FileId],
}

struct TypeEncoder<'a> {
    engine: &'a QueryEngine,
    checked: Arc<checking::CheckedModule>,
    package_by_file: &'a [(FileId, &'a str)],
    names: PrettyNames,
}

impl<'a> TypeEncoder<'a> {
    fn new(
        engine: &'a QueryEngine,
        checked: Arc<checking::CheckedModule>,
        package_by_file: &'a [(FileId, &'a str)],
    ) -> TypeEncoder<'a> {
        let names = PrettyNames::new();
        TypeEncoder { engine, checked, package_by_file, names }
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
                let value = self.engine.lookup_smol_str(value_id).to_string();
                schema::Type::String { kind: kind.into(), value }
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

struct ModuleEncoder<'a> {
    file_id: FileId,
    indexed: Arc<indexing::IndexedModule>,
    lowered: Arc<lowering::LoweredModule>,
    documented: Arc<documenting::DocumentedModule>,
    checked: Arc<checking::CheckedModule>,
    type_encoder: TypeEncoder<'a>,
}

impl<'a> ModuleEncoder<'a> {
    fn new(
        engine: &'a QueryEngine,
        file_id: FileId,
        package_by_file: &'a [(FileId, &'a str)],
    ) -> Result<(Option<String>, ModuleEncoder<'a>), Error> {
        let (parsed, _) = engine.parsed(file_id)?;
        let indexed = engine.indexed(file_id)?;
        let lowered = engine.lowered(file_id)?;
        let checked = engine.checked(file_id)?;
        let documented = engine.documented(file_id)?;

        let name = parsed.module_name().map(|name| name.to_string());
        let type_encoder = TypeEncoder::new(engine, Arc::clone(&checked), package_by_file);

        Ok((name, ModuleEncoder { file_id, indexed, lowered, documented, checked, type_encoder }))
    }

    fn encode_signature(&mut self, id: checking::TypeId) -> Result<schema::Type, Error> {
        self.type_encoder.encode_signature(id)
    }

    fn encode_term_items(
        &mut self,
        terms: impl IntoIterator<Item = indexing::TermItemId>,
    ) -> Result<Vec<schema::TermItem>, Error> {
        terms.into_iter().map(|term_id| self.encode_term_item(term_id)).collect()
    }

    fn encode_term_item(
        &mut self,
        term_id: indexing::TermItemId,
    ) -> Result<schema::TermItem, Error> {
        let term_item = &self.indexed.items[term_id];
        let term_documentation = self.documented.terms.get(&term_id);

        let name = term_item.name.as_ref().map(|name| name.to_string());
        let documentation = term_documentation.map(|term| term.documentation.to_string());
        let signature = term_signature(term_id, term_item, &self.checked)
            .map(|signature| self.type_encoder.encode_signature(signature))
            .transpose()?;

        Ok(schema::TermItem { name, documentation, signature, kind: term_kind(&term_item.kind) })
    }

    fn encode_type_item(
        &mut self,
        type_id: indexing::TypeItemId,
        instances: impl IntoIterator<Item = indexing::TermItemId>,
    ) -> Result<schema::TypeItem, Error> {
        let indexed = Arc::clone(&self.indexed);
        let (name, documentation, signature, kind, constructors, members, expansion) = {
            let type_item = &indexed.items[type_id];
            let type_documentation = self.documented.types.get(&type_id);

            let constructors = match &type_item.kind {
                indexing::TypeItemKind::Data { constructors, .. }
                | indexing::TypeItemKind::Newtype { constructors, .. } => constructors.as_slice(),
                _ => &[],
            };

            let members = match &type_item.kind {
                indexing::TypeItemKind::Class { members, .. } => members.as_slice(),
                _ => &[],
            };

            let name = type_item.name.as_ref().map(|name| name.to_string());
            let documentation = type_documentation.map(|t| t.documentation.to_string());
            let signature = self.checked.lookup_type(type_id);
            let kind = type_kind(&type_item.kind);
            let expansion = self.checked.lookup_synonym(type_id).map(|synonym| synonym.synonym);

            (name, documentation, signature, kind, constructors, members, expansion)
        };

        let signature = signature.map(|signature| self.encode_signature(signature)).transpose()?;
        let constructors = self.encode_term_items(constructors.iter().copied())?;
        let members = self.encode_term_items(members.iter().copied())?;
        let instances = self.encode_term_items(instances)?;
        let expansion = expansion.map(|synonym| self.encode_signature(synonym)).transpose()?;

        Ok(schema::TypeItem {
            name,
            documentation,
            signature,
            kind,
            constructors,
            members,
            instances,
            expansion,
        })
    }
}

pub fn render_package_manifest(
    engine: &QueryEngine,
    package: &PackageInput<'_>,
) -> Result<schema::Package, Error> {
    let mut modules = vec![];
    for &id in package.modules {
        let (parsed, _) = engine.parsed(id)?;
        if let Some(name) = parsed.module_name() {
            modules.push(name.to_string());
        }
    }

    Ok(schema::Package {
        name: package.name.to_string(),
        version: package.version.to_string(),
        license: package.license.map(str::to_string),
        description: package.description.map(str::to_string),
        dependencies: BTreeMap::clone(package.dependencies),
        modules,
    })
}

pub fn render_module(
    engine: &QueryEngine,
    file_id: FileId,
    package_by_file: &[(FileId, &str)],
) -> Result<Option<schema::Module>, Error> {
    let (name, mut encoder) = ModuleEncoder::new(engine, file_id, package_by_file)?;

    let Some(name) = name else { return Ok(None) };
    let documentation = Some(encoder.documented.documentation.to_string());

    let mut terms = vec![];
    let mut types = vec![];

    let mut nested_terms = NestedTerms::new();
    let mut instances_of = collect_instances_of(&encoder, &mut nested_terms);
    collect_constructors_members(&encoder, &mut nested_terms);

    let indexed = Arc::clone(&encoder.indexed);
    for (term_id, _) in indexed.items.iter_terms() {
        if nested_terms.contains(&term_id) {
            continue;
        }
        terms.push(encoder.encode_term_item(term_id)?);
    }
    for (type_id, _) in indexed.items.iter_types() {
        let instances = instances_of.remove(&type_id).unwrap_or_default();
        types.push(encoder.encode_type_item(type_id, instances)?);
    }

    Ok(Some(schema::Module { name, documentation, terms, types }))
}

fn collect_constructors_members(encoder: &ModuleEncoder<'_>, nested_terms: &mut NestedTerms) {
    for (_, type_item) in encoder.indexed.items.iter_types() {
        match &type_item.kind {
            indexing::TypeItemKind::Data { constructors, .. }
            | indexing::TypeItemKind::Newtype { constructors, .. } => {
                nested_terms.extend(constructors.iter().copied());
            }
            indexing::TypeItemKind::Class { members, .. } => {
                nested_terms.extend(members.iter().copied());
            }
            _ => {}
        }
    }
}

fn term_kind(kind: &indexing::TermItemKind) -> schema::TermKind {
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

fn type_kind(kind: &indexing::TypeItemKind) -> schema::TypeKind {
    match kind {
        indexing::TypeItemKind::Data { .. } => schema::TypeKind::Data,
        indexing::TypeItemKind::Newtype { .. } => schema::TypeKind::Newtype,
        indexing::TypeItemKind::Synonym { .. } => schema::TypeKind::Synonym,
        indexing::TypeItemKind::Class { .. } => schema::TypeKind::Class,
        indexing::TypeItemKind::Foreign { .. } => schema::TypeKind::Foreign,
        indexing::TypeItemKind::Operator { .. } => schema::TypeKind::Operator,
    }
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
        _ => checked.lookup_term(term_id),
    }
}

type NestedTerms = BTreeSet<indexing::TermItemId>;
type InstanceParentMap = BTreeMap<indexing::TypeItemId, Vec<indexing::TermItemId>>;
type InstanceParents = BTreeSet<indexing::TypeItemId>;

fn collect_instances_of(
    encoder: &ModuleEncoder<'_>,
    nested_terms: &mut NestedTerms,
) -> InstanceParentMap {
    let mut instances_by_parent = InstanceParentMap::new();

    for (term_id, term_item) in encoder.indexed.items.iter_terms() {
        if !matches!(
            term_item.kind,
            indexing::TermItemKind::Instance { .. } | indexing::TermItemKind::Derive { .. }
        ) {
            continue;
        }

        let parents = instance_parents(encoder, term_id, term_item);
        if parents.is_empty() {
            continue;
        }

        nested_terms.insert(term_id);
        for parent in parents {
            instances_by_parent.entry(parent).or_default().push(term_id);
        }
    }

    instances_by_parent
}

fn instance_parents(
    encoder: &ModuleEncoder<'_>,
    term_id: indexing::TermItemId,
    term_item: &indexing::TermItem,
) -> InstanceParents {
    let mut parents = InstanceParents::new();

    let checked_instance = match &term_item.kind {
        indexing::TermItemKind::Instance { id } => encoder.checked.lookup_instance(*id),
        indexing::TermItemKind::Derive { id } => encoder.checked.lookup_derived(*id),
        _ => None,
    };

    if let Some(instance) = checked_instance
        && let (parent_file, parent_type) = instance.resolution
        && parent_file == encoder.file_id
    {
        parents.insert(parent_type);
    }

    let Some(term_item) = encoder.lowered.info.get_term_item(term_id) else {
        return parents;
    };

    let arguments = match term_item {
        lowering::TermItemIr::Instance { arguments, .. }
        | lowering::TermItemIr::Derive { arguments, .. } => arguments,
        _ => return parents,
    };

    for &argument in arguments.iter() {
        collect_instance_type_parents(encoder, &mut parents, argument);
    }

    parents
}

fn collect_instance_type_parents(
    encoder: &ModuleEncoder<'_>,
    parents: &mut InstanceParents,
    type_id: lowering::TypeId,
) {
    let Some(kind) = encoder.lowered.info.get_type_kind(type_id) else { return };

    match kind {
        lowering::TypeKind::Constructor { resolution } => {
            if let Some((parent_file, parent_type)) = resolution
                && *parent_file == encoder.file_id
                && instance_type_parent(encoder, *parent_type)
            {
                parents.insert(*parent_type);
            }
        }
        lowering::TypeKind::ApplicationChain { function, arguments } => {
            if let Some(function) = function {
                collect_instance_type_parents(encoder, parents, *function);
            }
            for &argument in arguments.iter() {
                collect_instance_type_parents(encoder, parents, argument);
            }
        }
        lowering::TypeKind::Arrow { argument, result } => {
            if let Some(argument) = argument {
                collect_instance_type_parents(encoder, parents, *argument);
            }
            if let Some(result) = result {
                collect_instance_type_parents(encoder, parents, *result);
            }
        }
        lowering::TypeKind::Constrained { constraint, constrained } => {
            if let Some(constraint) = constraint {
                collect_instance_type_parents(encoder, parents, *constraint);
            }
            if let Some(constrained) = constrained {
                collect_instance_type_parents(encoder, parents, *constrained);
            }
        }
        lowering::TypeKind::Forall { inner, .. } => {
            if let Some(inner) = inner {
                collect_instance_type_parents(encoder, parents, *inner);
            }
        }
        lowering::TypeKind::Kinded { type_, kind } => {
            if let Some(type_) = type_ {
                collect_instance_type_parents(encoder, parents, *type_);
            }
            if let Some(kind) = kind {
                collect_instance_type_parents(encoder, parents, *kind);
            }
        }
        lowering::TypeKind::OperatorChain { head, tail } => {
            if let Some(head) = head {
                collect_instance_type_parents(encoder, parents, *head);
            }
            for pair in tail.iter() {
                if let Some(element) = pair.element {
                    collect_instance_type_parents(encoder, parents, element);
                }
            }
        }
        lowering::TypeKind::Record { items, tail } | lowering::TypeKind::Row { items, tail } => {
            for item in items.iter() {
                if let Some(type_) = item.type_ {
                    collect_instance_type_parents(encoder, parents, type_);
                }
            }
            if let Some(tail) = tail {
                collect_instance_type_parents(encoder, parents, *tail);
            }
        }
        lowering::TypeKind::Parenthesized { parenthesized } => {
            if let Some(parenthesized) = parenthesized {
                collect_instance_type_parents(encoder, parents, *parenthesized);
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

fn instance_type_parent(encoder: &ModuleEncoder<'_>, type_id: indexing::TypeItemId) -> bool {
    matches!(
        encoder.indexed.items[type_id].kind,
        indexing::TypeItemKind::Data { .. }
            | indexing::TypeItemKind::Newtype { .. }
            | indexing::TypeItemKind::Synonym { .. }
            | indexing::TypeItemKind::Foreign { .. }
    )
}
