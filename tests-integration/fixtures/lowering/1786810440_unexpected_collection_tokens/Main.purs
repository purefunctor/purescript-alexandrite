module Main where

arrayBinder [class] = 0
recordBinder { @ } = 0

arrayExpression = [class]
recordExpression = { @ }

type Row = ( value :: Int, @ )
type Record = { value :: Int, @ }
