module Main where

data Maybe a = Just a | Nothing

data List a = Cons a (List a) | Nil

infixr 6 Cons as :

binders (whole@(Just { name, value: (value :: Int) })) [true, false] (head : tail) =
  { whole, name, value, head, tail }

literals 42 = 1
literals (-1.5) = 2
literals "text" = 3
literals 'x' = 4
literals false = 5
literals _ = 6

guarded input
  | Just value <- input, value = value
  | true = fallback
  where
  fallback = 0
