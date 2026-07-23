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

unIdentity :: forall a. Identity a -> a
unIdentity value =
  let
    Identity inner = value
  in
    inner
