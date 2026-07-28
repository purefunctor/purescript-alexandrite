module Main where

import Lib (value, class Example, Data(Constructor), (<?>), type (++))
--            /            /      /                    /          /

valueUse = value

classUse :: forall a. Example a => a -> a
classUse a = a

constructorUse :: Data
constructorUse = Constructor

operatorUse = value <?> value

type TypeOperatorUse = Data ++ Data
