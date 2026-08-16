module Library (Source(..), class SourceClass, source, (<++>), type (:++:)) where

data Source = Source

class SourceClass a

source = Source

append left right = left
infixr 5 append as <++>

type Product left right = left
infixr 6 type Product as :++:
