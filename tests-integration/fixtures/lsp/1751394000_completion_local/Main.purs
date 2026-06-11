module Main where

vTop :: Int
vTop = 42

data Maybe a = Just a | Nothing

check :: Int -> { vPun :: Int } -> Int
check vBinder { vPun } = 
  let
    vLet :: Int
    vLet = 42 
  in
    v 
--   ^

check2 :: Int -> Maybe Int
check2 = J
--        ^

check3 :: Maybe Int
check3 = N
--        ^

type Check4 = M
--             ^

type Check5 localType = loc
--                         ^

class CheckClass a

instance checkInstance :: CheckClass implicitType => CheckClass imp
--                                                                 ^
