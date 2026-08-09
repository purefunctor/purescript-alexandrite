module Main where

identity :: forall a. a -> a
identity value = value

implicit :: Int
implicit = (identity :: forall a. a -> a) 0

monomorphic :: Int -> Int
monomorphic = (identity :: forall a. a -> a)

visibleIdentity :: forall @a. a -> a
visibleIdentity value = value

explicit :: Int
explicit = (visibleIdentity :: forall @a. a -> a) @Int 0
