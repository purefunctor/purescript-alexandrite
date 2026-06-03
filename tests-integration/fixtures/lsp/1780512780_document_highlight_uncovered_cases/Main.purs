module Main where

import Data.Eq (class Eq)

data Box = Box

derive instance eqBox :: Eq Box
--              &

binderPun = \binder -> { binder }
--                       &

letPun = let local = Box in { local }
--                            &

recordPunReference { field } = field
--                   &         &

plus left right = left

infixr 5 plus as <+>

operatorName = (<+>)
--              &

data Product a b = Product a b

infixr 6 type Product as :*:

type ProductName = (:*:)
--                 &
