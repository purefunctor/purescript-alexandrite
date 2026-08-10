module Main where

data Identity a = Identity a

letBinding :: forall a. a -> a
letBinding value =
  let
    local = value
  in
    local

whereBinding :: forall a. a -> a
whereBinding value = local
  where
  local :: a
  local = value

siblingPolymorphicBindings :: forall a. a -> { first :: a, second :: a }
siblingPolymorphicBindings value =
  let
    first :: forall b. b -> b
    first inner = inner

    second :: forall b. b -> b
    second inner = inner
  in
    { first: first value, second: second value }

unIdentity :: forall a. Identity a -> a
unIdentity value =
  let
    Identity inner = value
  in
    inner
