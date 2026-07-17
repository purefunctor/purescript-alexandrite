module Main where

foreign import data ForeignType :: Type
--                  /

foreign import foreignValue :: ForeignType
--             /

type Synonym = ForeignType
--   /

newtype Wrapper = Wrapper ForeignType
--      /

class Example a where
  member :: a -> a
-- /

unwrap (Wrapper value) = value
--      /

combine left right = left

infixl 5 combine as <?>

operatorUse = foreignValue <?> foreignValue
--                         /

infixl 5 type Synonym as :+:

type OperatorUse = Synonym :+: Synonym
--                         /
