module Main where

import Data.Bifunctor (class Bifunctor)
import Data.Foldable (class Foldable)
import Data.Functor (class Functor)
import Data.Functor.Contravariant (class Contravariant)
import Data.Profunctor (class Profunctor)
import Data.Traversable (class Traversable)

data CovariantRecord a = CovariantRecord { value :: a }
derive instance Functor CovariantRecord
derive instance Foldable CovariantRecord
derive instance Traversable CovariantRecord

data BivariantRecord a b = BivariantRecord { first :: a, second :: b }
derive instance Bifunctor BivariantRecord

data ContravariantRecord a = ContravariantRecord { predicate :: a -> Boolean }
derive instance Contravariant ContravariantRecord

data ProfunctorRecord a b = ProfunctorRecord { run :: a -> b }
derive instance Profunctor ProfunctorRecord
