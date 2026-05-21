module Main where

import Data.Reflectable (class Reflectable, reflectType)
import Type.Proxy (Proxy(..))

data Tag = Tag

instance Reflectable "hello" Tag where
  reflectType _ = Tag

test :: Tag
test = reflectType (Proxy :: Proxy "hello")
