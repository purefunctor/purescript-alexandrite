module Main where

data Choice a = Empty | One a | Pair a a

newtype Identity a = Identity a

data Nested a = Outer (Choice a)

first :: forall a. Choice a -> Choice a
first Empty = Empty
first (One value) = One value
first whole@(Pair left _) = case whole of
  Pair _ _ -> One left
  _ -> Empty

unwrap :: forall a. Identity a -> a
unwrap (Identity value) = value

nested :: forall a. Nested a -> Choice a
nested (Outer (One value)) = One value
nested _ = Empty

bind :: forall a b. a -> (a -> b) -> b
bind value continuation = continuation value

ordinaryBind :: Identity Int -> Int
ordinaryBind identity = bind identity (\(Identity value) -> value)
