module Main where

newtype Identity a = Identity a
--               &

data Proxy a = Proxy a
--         &

type Synonym a = a
--           &

class Example a where
--            &
  example :: a -> a

apply :: forall a. a -> a
--              &
apply value = value
