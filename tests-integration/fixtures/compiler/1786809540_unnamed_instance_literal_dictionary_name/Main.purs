module Main where

class NamedSymbol :: Symbol -> Constraint
class NamedSymbol symbol

instance NamedSymbol "foo bar"
