module Main where

data List a = Cons a (List a) | Nil

infixr 5 Cons as :

first :: forall a. List a -> a
first (value : _) = value

second :: forall a. List a -> a
second (_ : value : _) = value

inferred (value : _) = value

data Snoc a = Empty | Snoc (Snoc a) a

infixl 5 Snoc as :>

second' :: forall a. Snoc a -> a
second' (_ :> value :> _) = value
