module Main where

class Named a where
  name :: String

use :: forall body. Named body => body -> String
use _ = alias
  where
  alias = name @body
