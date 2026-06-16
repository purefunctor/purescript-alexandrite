module Main where

termHole :: Int -> Int
termHole argument =
  let
    localInt :: Int
    localInt = argument

    localString :: String
    localString = "not relevant"
  in
    ?term

type TypeHole a = ?type :: Type

recordPunHole :: { punInt :: Int, punString :: String } -> Int
recordPunHole { punInt, punString } = ?pun

class InstanceTypeHole implicit where
  instanceTypeHoleMember :: implicit -> implicit

instance InstanceTypeHole implicit where
  instanceTypeHoleMember :: ?implicit -> implicit
  instanceTypeHoleMember value = value
