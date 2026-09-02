module Main where

data Proxy a = Proxy

data Zero
data Succ number

type N0 = Zero
type N1 = Succ N0
type N2 = Succ N1
type N3 = Succ N2
type N4 = Succ N3
type N5 = Succ N4
type N6 = Succ N5
type N7 = Succ N6
type N8 = Succ N7
type N9 = Succ N8
type N10 = Succ N9
type N11 = Succ N10
type N12 = Succ N11
type N13 = Succ N12
type N14 = Succ N13
type N15 = Succ N14
type N16 = Succ N15
type N17 = Succ N16
type N18 = Succ N17
type N19 = Succ N18
type N20 = Succ N19
type N21 = Succ N20
type N22 = Succ N21
type N23 = Succ N22
type N24 = Succ N23
type N25 = Succ N24
type N26 = Succ N25
type N27 = Succ N26
type N28 = Succ N27
type N29 = Succ N28
type N30 = Succ N29
type N31 = Succ N30
type N32 = Succ N31
type N33 = Succ N32
type N34 = Succ N33
type N35 = Succ N34
type N36 = Succ N35
type N37 = Succ N36
type N38 = Succ N37
type N39 = Succ N38
type N40 = Succ N39

class Chain (number :: Type) where
  chain :: Int

instance chainZero :: Chain Zero where
  chain = 0
else instance chainNext :: Chain previous => Chain (Succ previous) where
  chain = observe 0

class Pair (number :: Type) where
  pair :: Proxy number -> Int

instance pairInstance :: (Chain number, Chain number) => Pair number where
  pair _ = 0

foreign import observe :: Int -> Int

result :: Int
result = pair (Proxy :: Proxy N40)
