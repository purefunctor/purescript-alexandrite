module Main where

arrayBinder [class] = 0
recordBinder { class } = 0

arrayExpression = [class]
recordExpression = { class }

type Row = ( class )
type Record = { class }
