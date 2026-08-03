module Main where

data Maybe a = Just a | Nothing

renameArgument argument =
--             /
  let shadowed = argument
  in argument
--   ?

renameLet input =
  let local :: Maybe Int -> Int
      local (Just item) = item
      local Nothing = input
  in local (Just input)
--   /

renameNamed whole@(Just item) = whole
--          /

renamePun { field } = field
--          /

punReference value = { value }
--                     /

renamePun' { renamed: original } = original
--                    /

punReference' original = { renamed: original }
--                                  /
