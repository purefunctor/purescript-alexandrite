module Main where

data Proxy a = Proxy

newtype Tagged t a = Tagged a

data Maybe :: Type -> Type
data Maybe a
  = Just a
  | Nothing
