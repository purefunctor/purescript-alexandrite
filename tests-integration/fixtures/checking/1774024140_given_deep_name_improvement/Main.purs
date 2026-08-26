module Main where

import Prim.Int (class Add)
import Prim.Row as Row
import Type.Proxy (Proxy(..))

needsAdd :: forall x. Add 1 x 3 => Proxy x
needsAdd = Proxy

nonHeadGivenImprovement
  :: forall n
   . Row.Union (a :: Proxy n) () (a :: Proxy 2)
  => Proxy n
nonHeadGivenImprovement = needsAdd

functionGivenImprovement
  :: forall value
   . Row.Union (a :: value -> String) () (a :: Int -> String)
  => Proxy value
functionGivenImprovement = Proxy

leftOpenRowGivenImprovement
  :: forall row
   . Row.Union (a :: Record (x :: Int | row)) () (a :: Record (x :: Int, y :: String))
  => Proxy row
leftOpenRowGivenImprovement = Proxy

rightOpenRowGivenImprovement
  :: forall row
   . Row.Union (a :: Record (x :: Int, y :: String)) () (a :: Record (x :: Int | row))
  => Proxy row
rightOpenRowGivenImprovement = Proxy

forceSolve =
  { nonHeadGivenImprovement
  , functionGivenImprovement
  , leftOpenRowGivenImprovement
  , rightOpenRowGivenImprovement
  }
