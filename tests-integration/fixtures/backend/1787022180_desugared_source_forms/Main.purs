module Main where

add :: Int -> Int -> Int
add left right = left

infixl 6 add as +

identity :: forall @a. a -> a
identity value = value

operatorApplication :: Int
operatorApplication = 1 + 2

visibleTypeApplication :: Int
visibleTypeApplication = identity @Int 42

increment :: Int -> Int
increment = _ + 1

accessValue :: forall value row. { value :: value | row } -> value
accessValue = _.value
