module Main where

foreign import await :: Int

arguments :: Int
arguments = await

default :: { "hyphen-label" :: Int }
default = { "hyphen-label": arguments }

readLabel :: { "hyphen-label" :: Int } -> Int
readLabel record = record."hyphen-label"

emptyLabel :: { "" :: Int }
emptyLabel = { "": arguments }

readEmptyLabel :: { "" :: Int } -> Int
readEmptyLabel record = record.""

data Tagged = Tagged { "hyphen-label" :: Int }

tagged :: Tagged
tagged = Tagged default
