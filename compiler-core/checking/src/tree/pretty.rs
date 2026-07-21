//! Implements the pretty printer for the checked semantic tree.

use building_types::QueryResult;
use files::FileId;
use indexing::{TermItem, TypeItem};
use pretty::{Arena, DocAllocator, DocBuilder};
use smol_str::{SmolStr, SmolStrBuilder};

use crate::CheckedModule;
use crate::core::Type;
use crate::core::pretty::{Pretty as TypePretty, PrettyQueries};
use crate::tree::{TermDeclarationKind, TypeDeclarationKind};

type Doc<'a> = DocBuilder<'a, Arena<'a>, ()>;

pub struct Pretty<'a, Q: ?Sized> {
    queries: &'a Q,
    width: usize,
    checked: &'a CheckedModule,
}

impl<'a, Q> Pretty<'a, Q>
where
    Q: PrettyQueries + ?Sized,
{
    pub fn new(queries: &'a Q, checked: &'a CheckedModule) -> Pretty<'a, Q> {
        Pretty { queries, width: 100, checked }
    }

    pub fn width(mut self, width: usize) -> Pretty<'a, Q> {
        self.width = width;
        self
    }

    pub fn render(&self, file_id: FileId) -> QueryResult<SmolStr> {
        let indexed = self.queries.indexed(file_id)?;
        let arena = Arena::new();
        let mut printer = Printer::new(&arena, self.queries, &indexed, self.checked, self.width);
        let document = printer.module();

        let mut output = SmolStrBuilder::new();
        document
            .render_fmt(self.width, &mut output)
            .expect("critical failure: failed to render checked semantic tree");
        Ok(output.finish())
    }
}

struct Printer<'arena, 'context, 'module, Q>
where
    Q: PrettyQueries + ?Sized,
{
    arena: &'arena Arena<'arena>,
    queries: &'context Q,
    indexed: &'module indexing::IndexedModule,
    checked: &'context CheckedModule,
    type_pretty: TypePretty<'context, Q>,
}

impl<'arena, 'context, 'module, Q> Printer<'arena, 'context, 'module, Q>
where
    Q: PrettyQueries + ?Sized,
{
    fn new(
        arena: &'arena Arena<'arena>,
        queries: &'context Q,
        indexed: &'module indexing::IndexedModule,
        checked: &'context CheckedModule,
        width: usize,
    ) -> Printer<'arena, 'context, 'module, Q> {
        let type_pretty = TypePretty::new(queries, checked).without_rigid_kinds().width(width);
        Printer { arena, queries, indexed, checked, type_pretty }
    }

    fn module(&mut self) -> Doc<'arena> {
        let mut declarations = vec![];

        for (type_id, TypeItem { name, .. }) in self.indexed.items.iter_types() {
            let Some(name) = name else { continue };
            let Some(declaration_id) = self.checked.tree.lookup_type_declaration(type_id) else {
                continue;
            };

            let declaration = &self.checked.tree[declaration_id];
            let keyword = match &declaration.declaration {
                TypeDeclarationKind::Data(_) => "data",
                TypeDeclarationKind::Newtype(_) => "newtype",
            };
            let kind = declaration.kind;

            self.type_pretty.reset();
            let signature = self.type_pretty.render_kind_signature(name, kind);
            let signature = self.arena.text(format!("{keyword} {signature}"));

            let declaration = self.data_declaration(type_id, keyword, name);

            declarations.push(signature.append(self.arena.hardline()).append(declaration));
        }

        let mut declarations = declarations.into_iter();
        if let Some(first) = declarations.next() {
            declarations.fold(first, |document, declaration| {
                document
                    .append(self.arena.hardline())
                    .append(self.arena.hardline())
                    .append(declaration)
            })
        } else {
            self.arena.nil()
        }
    }

    fn data_declaration(
        &mut self,
        type_id: indexing::TypeItemId,
        keyword: &str,
        name: &str,
    ) -> Doc<'arena> {
        let declaration_id = self
            .checked
            .tree
            .lookup_type_declaration(type_id)
            .expect("invariant violated: missing checked type declaration");
        let declaration = &self.checked.tree[declaration_id];
        let data = match &declaration.declaration {
            TypeDeclarationKind::Data(data) | TypeDeclarationKind::Newtype(data) => data,
        };

        let mut parameter_names = vec![];
        for &parameter in data.parameters.iter() {
            let parameter = self.queries.lookup_forall_binder(parameter);
            let name = self.type_pretty.display_name(parameter.name);
            parameter_names.push((name.to_string(), parameter.visible));
        }

        let mut head = self.arena.text(format!("{keyword} {name}"));
        for (parameter, visible) in &parameter_names {
            let parameter = if *visible { format!("@{parameter}") } else { parameter.to_string() };
            head = head.append(self.arena.text(format!(" {parameter}")));
        }

        let mut declaration = head;
        let constructors = self.indexed.data_constructors(type_id);
        for (&declaration_id, constructor_id) in data.constructors.iter().zip(constructors) {
            let TermItem { name: constructor_name, .. } = &self.indexed.items[constructor_id];
            let Some(constructor_name) = constructor_name else { continue };
            let constructor = &self.checked.tree[declaration_id];
            let TermDeclarationKind::Constructor(constructor) = &constructor.kind else {
                unreachable!("invariant violated: data declaration contains a value declaration");
            };

            let mut result = self.arena.text(name.to_string());
            for (parameter, _) in &parameter_names {
                result = result.append(self.arena.text(format!(" {parameter}")));
            }

            let mut constructor_type = result;
            for &argument_id in constructor.arguments.iter().rev() {
                let argument = self.type_pretty.render(argument_id);
                let argument = match self.queries.lookup_type(argument_id) {
                    Type::Forall(..)
                    | Type::Constrained(..)
                    | Type::Function(..)
                    | Type::Kinded(..) => format!("({argument})"),
                    _ => argument.to_string(),
                };

                let result = constructor_type;
                constructor_type = self
                    .arena
                    .text(argument)
                    .append(self.arena.text(" ->"))
                    .append(self.arena.line().append(result).nest(2))
                    .group();
            }

            let constructor_type = self.arena.line().append(constructor_type).nest(4);
            let constructor = self
                .arena
                .text(format!("  | {constructor_name} ::"))
                .append(constructor_type)
                .group();
            declaration = declaration.append(self.arena.hardline()).append(constructor);
        }

        declaration
    }
}
