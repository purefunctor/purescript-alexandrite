module Library (append, Product, (<>), type (:*:)) where

append left _ = left

infixr 5 append as <>

type Product a b = a

infixr 6 type Product as :*:
