module Main where

foreign import node :: String -> Array String -> Array String -> String

foreign import attribute :: String -> String

foreign import text :: String -> String

render :: Boolean -> String
render state =
  let
    dynamicClass value =
      if value then "active" else "inactive"
  in
    node "main"
      [ attribute "root" ]
      [ node "span"
          [ attribute (dynamicClass state) ]
          [ text "first" ]
      , node "span" [] [ text "second" ]
      ]
