module Main where

import Library (External(..))

data Local a b = Local a b

data Empty = Empty

local :: Local Int String
local = Local 1 "local"

partial :: forall a. a -> Local Int a
partial = Local 2

empty :: Empty
empty = Empty

external :: External String
external = External "external"
