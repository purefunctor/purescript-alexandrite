module Main where

class Select a where
  select :: a -> a -> a

instance selectInt :: Select Int where
  select first second = second

data Wrapped a = Wrapped a

instance selectWrapped :: Select a => Select (Wrapped a) where
  select (Wrapped first) (Wrapped second) = Wrapped (select first second)

ordinaryPrerequisite :: Wrapped Int
ordinaryPrerequisite = select (Wrapped 10) (Wrapped 20)
