module Main where

newtype Identity a = Identity a

unwrap :: forall a. Identity a -> a
unwrap wrapped =
  let
    Identity value = wrapped
  in
    value

select :: { first :: Int, second :: String } -> String
select record =
  let
    { first, second } = record
  in
    second
