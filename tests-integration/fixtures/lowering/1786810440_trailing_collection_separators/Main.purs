module Main where

arrayBinder [value,] = value
recordBinder { value, } = value

arrayExpression = [1,]
recordExpression = { value: 1, }

type Row = ( value :: Int, )
type Record = { value :: Int, }
