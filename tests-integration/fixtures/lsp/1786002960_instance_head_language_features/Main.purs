module Main where

import Lib (class Functor, class Original)
import Lib as L
import Lib as FunctorModule
import HiddenLib hiding (class Hidden)

newtype Box a = Box a

data Collision

data Renamed

class Example a where
--    /
  example :: a

instance Functor Box where
--       @$%
  map function (Box value) = Box (function value)

instance Fu Box
--       @ ^

instance L.Fu Box
--           ^

instance Coll Box
--           ^

instance Hid Box
--          ^

instance Partial
--       %

foreign import requiresPartial :: Prim.Partial => Int

instance Prim.Par
--               *

instance Original Box
--       /
