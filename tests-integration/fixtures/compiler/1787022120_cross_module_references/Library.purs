module Library where

data Box a = Box a

libraryValue :: Int
libraryValue = 42

box :: Box Int
box = Box libraryValue
