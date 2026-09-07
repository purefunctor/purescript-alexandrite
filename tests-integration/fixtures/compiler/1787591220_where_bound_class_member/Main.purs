module Main where

class Named a where
  name :: a -> String

instance Named String where
  name value = value

use :: forall a. Named a => a -> String
use value = alias value
  where
  alias = name

useLet :: forall a. Named a => a -> String
useLet value =
  let
    alias = name
  in
    alias value
