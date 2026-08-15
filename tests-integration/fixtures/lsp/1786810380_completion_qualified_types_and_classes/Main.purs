module Main where

import Library as Library

type QualifiedType = Library.B
--                            ^

instance qualifiedClass :: Library.W
--                                  ^
