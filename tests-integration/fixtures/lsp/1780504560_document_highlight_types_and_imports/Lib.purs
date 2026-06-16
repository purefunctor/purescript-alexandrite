module Lib where

data Box = Box

class ImportedClass a where
  importedMember :: a -> Box

foreign import data Product :: Type -> Type -> Type

infixr 6 type Product as :*:

value :: Box
value = Box

append :: Box -> Box -> Box
append left right = left

infixr 5 append as <++>
