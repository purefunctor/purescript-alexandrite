module Main where

foreign import data Pair :: Type -> Type -> Type
foreign import data SymbolProxy :: Symbol -> Type
foreign import data IntProxy :: Int -> Type

infixr 6 type Pair as :*:

class Convert input output | input -> output

foreign import convert :: forall input output. Convert input output => input -> output

type Nested left right = left :*: (right :*: left)

symbols :: SymbolProxy "hello"
symbols = symbols

integers :: IntProxy 42
integers = integers

record :: forall row. { value :: Int | row } -> (value :: Int | row)
record value = value
