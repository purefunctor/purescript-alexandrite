module Main where

type Model =
  { count :: Int
  , nested :: { enabled :: Boolean, label :: String }
  }

model :: Model
model = { count: 0, nested: { enabled: true, label: "before" } }

updated :: Model
updated = model { count = 1, nested { enabled = false, label = "after" } }
