module Main where

newtype Identity a = Identity a
--               $

data Proxy :: forall kind. kind -> Type
data Proxy a = Proxy
--         $
