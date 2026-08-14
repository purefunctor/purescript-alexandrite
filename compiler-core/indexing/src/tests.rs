use la_arena::RawIdx;

use crate::{IndexedModule, TermItemId, index_module};

fn index(source: &str) -> IndexedModule {
    let lexed = lexing::lex(source);
    let tokens = lexing::layout(&lexed);
    let (parsed, errors) = parsing::parse(&lexed, &tokens);
    assert!(errors.is_empty(), "test module must parse: {errors:?}");

    let root = parsed.syntax_node();
    let stabilized = stabilizing::stabilize_module(&root);
    index_module(source, &parsed.cst(), &stabilized)
}

#[test]
fn constructor_owners_preserve_type_and_constructor_ids() {
    let indexed = index(
        "module Main where\n\
         data First = FirstA | FirstB\n\
         newtype Second = Second Int\n\
         value = 0\n",
    );

    let first_type = indexed.names.types.lookup("First").unwrap();
    let second_type = indexed.names.types.lookup("Second").unwrap();
    let first_a = indexed.names.terms.lookup("FirstA").unwrap();
    let first_b = indexed.names.terms.lookup("FirstB").unwrap();
    let second = indexed.names.terms.lookup("Second").unwrap();
    let value = indexed.names.terms.lookup("value").unwrap();

    let first_constructors = indexed.data_constructors(first_type);
    let first_constructors = first_constructors.collect::<Vec<_>>();
    assert_eq!(first_constructors, [first_a, first_b]);
    assert_eq!(indexed.data_constructors(second_type).next(), Some(second));
    assert_eq!(indexed.constructor_type(first_a), Some(first_type));
    assert_eq!(indexed.constructor_type(first_b), Some(first_type));
    assert_eq!(indexed.constructor_type(second), Some(second_type));
    assert_eq!(indexed.constructor_type(value), None);
    let out_of_range = TermItemId::from_raw(RawIdx::from_u32(u32::MAX));
    assert_eq!(indexed.constructor_type(out_of_range), None);
}
