{- This file implements the frontend to the MECIS web service.
-}


module Main exposing (..)

import Html exposing (Html, Attribute, input, text, div, br, p, a,b,span, img,h1,header,label,select, option)
import Html.Events exposing (onInput, on, keyCode)
import Html.Attributes exposing (href, class, style, src, alt, value, id)
import Material
import Material.Scheme
import Material.Button as Button
-- import Material.Select as Select
import Material.Options as Options exposing (css)
import Material.Tabs as Tabs
import Material.Icon as Icon exposing (..)
import Material.Footer as Footer
import Material.Table as Table exposing (table, thead, tbody, tr, th, td)
import Material.Chip as Chip exposing (content)
import Material.Progress as Loading

import Http
import Json.Decode as Decode
import Json.Decode.Pipeline exposing (decode, required)


-- MODEL

type alias MecisInfo =
    { organisms : List String
    , models : List String
    , inreacs : List String
    , exreacs : List String
    , mbys : List Float
    , mpys : List Float
    , scens : List Int
    , reactions : List String
    }

type alias ResponseRow =
    { organism : String
    , model : String
    , inreac : String
    , exreac : String
    , mby : Float
    , mpy : Float
    , scen : Int
    , mis : List KnockOut
    }

type alias KnockOut =
    { name : String
    , link : String
    }

type alias Model =
    { ctab : Int
    , context : MecisInfo
    , mdl : Material.Model
    , cmessage : String
    , corganism : String
    , cmodel : String
    , cinreac : String
    , cexreac : String
    , cmby : String
    , cmpy : String
    , cscen : String
    , cmustin: String
    , cmustins: List String
    , cforbidden: String
    , cforbiddens: List String
    , morepressed : Bool
    , col_offset : Int
    , max_mis : Int
    , rows : List ResponseRow
    }
model : Model
model =
    { ctab = 0
    , context =     { organisms = [ ]        
                    , models = []                  
                    , inreacs = []                  
                    , exreacs = []                  
                    , mbys = []                  
                    , mpys = []
                    , scens = []
                    , reactions = []         
                    }
    , mdl =  Material.model
    
    , cmessage = ""
    , corganism = ""
    , cmodel = ""
    , cinreac = "None"
    , cexreac = "None"
    , cmby = "NaN"
    , cmpy = "NaN"
    , cscen = "1"
    , cmustin = ""
    , cmustins = []    
    , cforbidden = ""
    , cforbiddens = []  
    , morepressed = False
    , col_offset = 0
    , max_mis = 0
    , rows = []
    }



-- ACTION, UPDATE


type Msg
    = Init (Result Http.Error MecisInfo)
    | AboutTab
    | SearchTab
    | ChangeOrganism (String)
    | ChangeModel (String)
    | ChangeInreac (String)
    | ChangeExreac (String)
    | ChangeMby (String)
    | ChangeMpy (String)
    | ChangeScen (String)
    | ChangeMustin (String)
    | MustInKeyDown (Int)
    | RemoveMustin (Int)
    | ChangeForbidden (String)
    | ForbiddenKeyDown (Int)
    | RemoveForbidden (Int)
    | Submit
    | SubmitMore
    | NewData (Result Http.Error Response1)
    | MoreData (Result Http.Error Response1)
    | Mdl (Material.Msg Msg)


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
    
        Init (Ok response) ->
            case (List.head response.organisms) of
                Just(org) -> 
                    case (List.head response.models) of
                         Just(m) ->
                            ( {model | context = response, corganism = org, cmodel = m }
                            , Cmd.none
                            )
                         _ -> ( {model | context = response }, Cmd.none)
                _ -> ( {model | context = response }, Cmd.none)
            
        Init (Err blub) ->
            ( {model | cmessage = toString blub }
            , Cmd.none
            )
        SearchTab ->
            ( { model | ctab = 0 } -- TODO force update
            , Cmd.none
            )
        AboutTab ->
            ( { model | ctab = 1 }
            , Cmd.none
            )            
        ChangeOrganism (organism) ->
            ( { model | corganism = organism }
            , Cmd.none
            )            
        ChangeModel (m) ->
            ( { model | cmodel = m }
            , Cmd.none
            )            
        ChangeInreac (inreac) ->
            ( { model | cinreac = inreac }
            , Cmd.none
            )            
        ChangeExreac (exreac) ->
            ( { model | cexreac = exreac }
            , Cmd.none
            )            
        ChangeMby (mby) ->
            ( { model | cmby = mby }
            , Cmd.none
            )            
        ChangeMpy (mpy) ->
            ( { model | cmpy = mpy }
            , Cmd.none
            )            
        ChangeScen (scen) ->
            ( { model | cscen = scen }
            , Cmd.none
            )   
        ChangeMustin (reac) ->
            ( { model | cmustin = reac}
            , Cmd.none
            )   
        ChangeForbidden (reac) ->
            ( { model | cforbidden = reac}
            , Cmd.none
            )      
        MustInKeyDown (key) ->
            if key == 13 && List.member model.cmustin model.context.reactions
            then ({ model | cmustins = model.cmustin::model.cmustins, cmustin ="" } , Cmd.none)
            else (model, Cmd.none)
        ForbiddenKeyDown (key) ->
            if key == 13 && List.member model.cforbidden model.context.reactions
            then ({ model | cforbiddens = model.cforbidden::model.cforbiddens, cforbidden ="" } , Cmd.none)
            else (model, Cmd.none)
        RemoveMustin (idx) ->
            ( { model | cmustins = (List.take (idx-1) model.cmustins)++(List.drop idx model.cmustins) }
            , Cmd.none
            )            
        RemoveForbidden (idx) ->
            ( { model | cforbiddens = (List.take (idx-1) model.cforbiddens)++(List.drop idx model.cforbiddens) }
            , Cmd.none
            )            
        Submit ->
            ( { model | cmessage = "Processing query!"}
            , getData model
            )            
        SubmitMore ->
            ( {model | morepressed = True}
            , getMoreData model
            )
            
        NewData (Ok resp) ->
            ( {model | cmessage =(toString resp.max_mis)++" intervention sets found!", 
                       rows = resp.rows, 
                       col_offset = resp.col_offset, 
                       max_mis = resp.max_mis }
            , Cmd.none
            )
        NewData (Err blub) ->
            ( {model | cmessage = toString blub }
            , Cmd.none
            )

        MoreData (Ok response) ->
            ( {model | morepressed = False, rows = (List.append model.rows response.rows), col_offset = response.col_offset }
            , Cmd.none
            )
        MoreData (Err blub) ->
            ( {model | cmessage = toString blub, morepressed = False }
            , Cmd.none
            )
        -- Boilerplate: Mdl action handler.
        Mdl msg_ ->
            Material.update Mdl msg_ model
        



-- VIEW


type alias Mdl =
    Material.Model


view : Model -> Html Msg
view model =
    span []
        [ mheader
        , mtabs model     
        , footer
        ]
        |> Material.Scheme.top



-- Load Google Mdl CSS. You'll likely want to do that not in code as we
-- do here, but rather in your master .html file. See the documentation
-- for the `Material` module for details.


main : Program Never Model Msg
main =
    Html.program
        { init = ( model, getMecisInfo )
        , view = view
        , subscriptions = always Sub.none
        , update = update
        }
        

onKeyDown : (Int -> msg) -> Attribute msg
onKeyDown tagger =
  on "keydown" (Decode.map tagger keyCode)        
        
mtabs : Model -> Html Msg
mtabs model =
        Tabs.render Mdl [0] model.mdl
        [ Tabs.ripple
        , Tabs.onSelectTab selectTab 
        , Tabs.activeTab model.ctab
        ]
        [ Tabs.label
            [ Options.center ]
            [ Icon.i "search"
            , Options.span [ css "width" "4px" ] []
            , text "Search"
            ]
        , Tabs.label
            [ Options.center ]
            [ Icon.i "info_outline"
            , Options.span [ css "width" "4px" ] []
            , text "About"
            ]
        ]
        [ case model.ctab of
            0 -> searchTab model
            1 -> aboutTab
            _ -> searchTab model
        ]
 
selectTab i = 
    case i of
        0 -> SearchTab
        1 -> AboutTab
        _ -> SearchTab
 
searchTab : Model -> Html Msg 
searchTab model = 
    div [] [ myform model
           , message_area model 
           , result_area model
           ]
        
aboutTab = div [fstyle] [ text ("This is the KISS ME - web service.") ]
 
mheader : Html Msg
mheader = header []
    [ h1 [hstyle] 
         [ img [ src (server++"/mecis_logo"),
                 alt "Logo" 
               ] [ ],
           span [style[("padding-left","2em")]] [text ("KISS ME - web service")]
         ] 
    ]
                    
footer = Footer.mini []
    { left =
        Footer.left []
            [ Footer.links []
                [ Footer.linkItem [ Footer.href "http://www.mpi-magdeburg.mpg.de" ] [ Footer.html <| text "mpi-magdeburg"]
                ]
            ]

    , right =
        Footer.right [] []
    }                    

message_area : Model -> Html Msg
message_area model = 
   case model.cmessage of
    "" -> text ""
    string -> span [style [("padding-left","2em")]] [ b [] [text string] ]


server = "http://www2.mpi-magdeburg.mpg.de/projects/mecis"    
    
result_area : Model -> Html Msg
result_area model = 
    let len = List.length model.rows
    in
    if model.cmessage == "Processing query!"
    then  div [fstyle] [ Loading.indeterminate, br [] []]
    else
        if len > 0
        then div [fstyle] [ p []
                              [
--                               b [spsty] [text ((toString model.max_mis)++" intervention sets found!")],
                                if model.max_mis > 10000
                                then                                
                                    Button.render Mdl [9, 0, 0, 1] model.mdl
                                    [ Button.ripple
                                    , Button.colored
                                    , Button.raised
                                    , Button.link (getDlLink model)
                                    , Options.attribute <| (Html.Attributes.downloadAs "KISSME-MIS.csv")
                                    ]
                                    [ text "Download first 10000 as CSV" ]
                                else
                                    Button.render Mdl [9, 0, 0, 1] model.mdl
                                    [ Button.ripple
                                    , Button.colored
                                    , Button.raised
                                    , Button.link (getDlLink model)
                                    , Options.attribute <| (Html.Attributes.downloadAs "KISSME-MIS.csv")
                                    ]
                                    [ text "Download as CSV" ]
                              ],
                            printtable model.rows,
                            if (List.length model.rows) >= model.max_mis || (model.morepressed)
                            then span [] []
                            else
                                Button.render Mdl
                                [ 1 ]
                                model.mdl
                                [ Button.raised,
                                Button.colored,
                                Button.ripple,
                                Options.onClick SubmitMore ]
                                [ text "More!" ]
                        ]
                
         
                            else span [] []

  
printtable: List ResponseRow -> Html Msg
printtable rows = table [] (List.append [printtablehead] (printrows rows))

printtablehead : Html Msg
printtablehead = 
    tr [] 
       [(th [] [text "Organism"]),
        (th [] [text "Model"]),
        (th [] [text "Intake reaction"]),
        (th [] [text "Excretion reaction"]),
        (th [] [text "Min biomass yield"]),
        (th [] [text "Min product yield"]),
        (th [] [text "Scenario"]),
        (th [] [text "Intervention set"])
       ]

printrows: List ResponseRow -> List (Html Msg)        
printrows rows =
    case rows of
        [] -> []
        x::xs -> (tr [] 
                     [ (td [] [text x.organism]),
                       (td [] [text x.model]),
                       (td [] [text x.inreac]),
                       (td [] [text x.exreac]),
                       (td [] [text (toString x.mby)]),
                       (td [] [text (toString x.mpy)]),
                       (td [] [text (toString x.scen)]),
                       (td [ (Options.css "text-align" "left")] [span [] (printmis x.mis)])
                     ]) :: (printrows xs)      
                    
printmis : List KnockOut-> List (Html Msg)
printmis mis = 
    case mis of 
        [] -> [text ""]
        x::xs -> (printko x)::(printmis xs)
        
printko: KnockOut -> Html Msg
printko ko = 
    case ko.link of
        "" -> text (ko.name++" ")
        url -> span [] 
                    [ a [href url] [text (ko.name)],
                      br [] [] 
                    ]
        
myform : Model ->  Html Msg
myform m =
    let ctx = m.context
    in
    div [fstyle] [
          p [] [b [] [text ("Please select the parameters for the desired intervention set and press the submit button.")]]
        , p []
            [ label [spsty] [text ("Organism:")],
              select [onInput ChangeOrganism, value m.corganism]  ( ctx.organisms |> List.map (\string ->
                                 (option [value string] [text string]) 
                                 )
                        ),
              label [spsty] [text ("Model:")],
              select [onInput ChangeModel, value m.cmodel] (ctx.models |> List.map (\string ->
                              (option [value string] [text string]) 
                              )
                        ) 
            ]
        , p []
            [ label [spsty] [text ("Substrate intake reaction:")],
              select [onInput ChangeInreac, value m.cinreac] ((option [value "None"] [text "undefined"])
                         ::
                         (ctx.inreacs |> List.map (\string ->
                                (option [value string] [text string]) 
                             )
                         )
                        ),
              label [spsty] [text ("Product excretion reaction:")],
              select [onInput ChangeExreac, value m.cexreac] ((option [value "None"] [text "undefined"])
                         ::
                         (ctx.exreacs |> List.map (\string ->
                                (option [value string] [text string]) 
                             )
                         )
                        )
            ]
        , p []
            [ label [spsty] [text ("Minimal biomass yield:")],
              select [onInput ChangeMby, value m.cmby] ((option [value "NaN"] [text "undefined"])
                         ::
                         (ctx.mbys |> List.map (\float ->
                               (option [value (toString float)] [text (toString float)]) 
                            )
                         )
                        ),
              label [spsty] [text ("Minimal product yield:")],
              select [onInput ChangeMpy, value m.cmpy] ((option [value "NaN"] [text "undefined"])
                         ::
                         (ctx.mpys |> List.map (\float ->
                               (option [value (toString float)] [text (toString float)]) 
                            )
                         )
                        ),
              label [spsty] [text ("Scenario:")],
              select [onInput ChangeScen, value m.cscen] (ctx.scens |> List.map (\int ->
                            (option [value (toString int)] [text (toString int)]) 
                            )
                        ) 
            ],
          Html.datalist [id "dl1"] (mydl ctx.reactions)
        , mustinform m,
          forbiddenform m
        , p [spsty] [Button.render Mdl
                     [ 1 ]
                     model.mdl
                    [ Button.raised,
                      Button.colored,
                      Button.ripple,
                      Options.onClick Submit ]
                    [ text "Submit Query!" ]
            ]
        ]
mydl: List String -> List (Html Msg)
mydl list =
    case list of
         [] -> []
         x::xs -> (option [value x] [text x])::(mydl xs)

mustinform: Model -> Html Msg
mustinform model =
    p []
      [ label [spsty] [text ("These reactions must be included:")],
        input [onInput ChangeMustin , Html.Attributes.list "dl1", value model.cmustin
              , onKeyDown MustInKeyDown
              ] []
      , br [] []
      , drawmustins model.cmustins 1  

      ]
                    
forbiddenform: Model -> Html Msg
forbiddenform model =
    p []
      [ label [spsty] [text ("These reactions are not allowed:")],
        input [onInput ChangeForbidden, value model.cforbidden, Html.Attributes.list "dl1"
              , onKeyDown ForbiddenKeyDown
              ] []
      , br [] []
      , drawforbiddens model.cforbiddens 1
      ]
        
      
drawmustins: List String -> Int -> Html Msg
drawmustins list i =
    case list of
         [] -> span [] []
         x::xs ->   span [] [ Chip.span
                    [ Options.css "margin" "5px 5px"
                    , Options.css "backgroundColor" "#7ebd00"
                    -- TODO , Options.css "text-color" "white"
                    , Options.onClick (RemoveMustin i)
                    , Chip.deleteClick (RemoveMustin i),
                      Chip.deleteIcon "cancel"
                    ]
                    [ Chip.content []
                        [ text (x) ]
                    ]
                    , drawmustins xs (i+1)
                    ]

drawforbiddens: List String -> Int -> Html Msg
drawforbiddens list i =
    case list of
         [] -> span [] []
         x::xs ->   span [] [ Chip.span
                    [ Options.css "margin" "5px 5px"
                    , Options.css "backgroundColor" "#e14a3d"
                    -- TODO , Options.css "text-color" "white"
                    , Options.onClick (RemoveForbidden i)
                    , Chip.deleteClick (RemoveForbidden i),
                      Chip.deleteIcon "cancel"
                    ]
                    [ Chip.content []
                        [ text (x) ]
                    ]
                    , drawforbiddens xs (i+1)
                    ]

        
hstyle : Attribute msg
hstyle = style
    [ ("margin-top", "0")
    , ("padding", "2rem" )
    , ("backgroundColor", "#31011b")
    , ("color", "#ff55ff")
    ]
fstyle = style [("padding", "2em" )]
spsty = style [("padding-right","1em"),("padding-left","1em")]

-- HTTP

type alias Response1 =
    { col_offset :Int
    , max_mis : Int
    , rows : List ResponseRow
    }


getDlLink : Model -> String
getDlLink model =
  server++"/getcsv?organism="++model.corganism
        ++"&model="++model.cmodel
        ++"&inreac="++model.cinreac
        ++"&exreac="++model.cexreac
        ++"&mby="++model.cmby
        ++"&mpy="++model.cmpy
        ++"&scen="++model.cscen
        ++"&mustin="++(list2string model.cmustins)
        ++"&forbidden="++(list2string model.cforbiddens)
        ++"&col_offset=0"
    
getData : Model -> Cmd Msg
getData model =
  let
    url =server++"/getcis?organism="++model.corganism
        ++"&model="++model.cmodel
        ++"&inreac="++model.cinreac
        ++"&exreac="++model.cexreac
        ++"&mby="++model.cmby
        ++"&mpy="++model.cmpy
        ++"&scen="++model.cscen
        ++"&mustin="++(list2string model.cmustins)
        ++"&forbidden="++(list2string model.cforbiddens)
        ++"&col_offset=0"
  in
    Http.send NewData (Http.get url resp1Decoder)
    
getMoreData : Model -> Cmd Msg
getMoreData model =
  let
    url =server++"/getcis?organism="++model.corganism
        ++"&model="++model.cmodel
        ++"&inreac="++model.cinreac
        ++"&exreac="++model.cexreac
        ++"&mby="++model.cmby
        ++"&mpy="++model.cmpy
        ++"&scen="++model.cscen
        ++"&mustin="++(list2string model.cmustins)
        ++"&forbidden="++(list2string model.cforbiddens)
        ++"&col_offset="++(toString model.col_offset)
  in
    Http.send MoreData (Http.get url resp1Decoder)    
    
list2string : List String ->String
list2string l =
    case l of
         [] -> ""
         x::xs -> x++" "++list2string xs
    
resp1Decoder : Decode.Decoder Response1
resp1Decoder =
  decode Response1
    |> required "col_offset" Decode.int
    |> required "max_mis" Decode.int  
    |> required "rows" decodeResponseRows

decodeResponseRows : Decode.Decoder (List ResponseRow)
decodeResponseRows = Decode.list decodeResponseRow  


decodeResponseRow : Decode.Decoder ResponseRow
decodeResponseRow =
    decode ResponseRow
    |> required "organism" Decode.string
    |> required "model" Decode.string  
    |> required "inreac" Decode.string 
    |> required "exreac" Decode.string
    |> required "mby" Decode.float
    |> required "mpy" Decode.float
    |> required "scen" Decode.int
    |> required "mis" decodeMis

decodeMis : Decode.Decoder (List KnockOut)
decodeMis = Decode.list decodeKnockOut  

decodeKnockOut : Decode.Decoder KnockOut
decodeKnockOut =
    decode KnockOut
    |> required "name" Decode.string
    |> required "link" Decode.string  


getMecisInfo :  Cmd Msg
getMecisInfo =
  let
    url = server++"/mecisinfo"
  in
    Http.send Init (Http.get url decodeMecisInfo)      
    
decodeMecisInfo : Decode.Decoder MecisInfo
decodeMecisInfo =
  decode MecisInfo
    |> required "organisms" decode_string_list
    |> required "models" decode_string_list  
    |> required "inreacs" decode_string_list 
    |> required "exreacs" decode_string_list
    |> required "mbys" (Decode.list Decode.float)
    |> required "mpys" (Decode.list Decode.float)
    |> required "scens" (Decode.list Decode.int)
    |> required "reactions" decode_string_list
    
decode_string_list : Decode.Decoder (List String)    
decode_string_list = Decode.list Decode.string    
