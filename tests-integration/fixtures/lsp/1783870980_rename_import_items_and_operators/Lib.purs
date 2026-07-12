module Lib (value, class Example, Data(Constructor), (<?>), type (++)) where

value = 42

class Example a

data Data = Constructor

infix 5 value as <?>

infix 5 type Data as ++
