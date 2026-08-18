module Main where

foreign import data Opaque :: Type

foreign import foreignValue :: Opaque
foreign import foreignFunction :: Opaque -> Int

value :: Int
value = foreignFunction foreignValue
