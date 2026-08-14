module Main where

data Box a = Box a | Other

test (Box _) = 0
test Box = 1

test2 Box = 0
test2 (Box _) = 1
