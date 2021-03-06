module Api.Endpoint exposing (Endpoint, loadAgents, article, articles, comment, comments, favorite, feed, follow, login, profiles, request, tags, user, users, discover, listing, stignore, flush)

import Agents exposing (Agent)
import ContentTypes exposing (ContentType)

import Article.Slug as Slug exposing (Slug)
import CommentId exposing (CommentId)
import Http
import Url.Builder exposing (QueryParameter)
import Username exposing (Username)


{-| Http.request, except it takes an Endpoint instead of a Url.
-}
request :
    { body : Http.Body
    , expect : Http.Expect a
    , headers : List Http.Header
    , method : String
    , timeout : Maybe Float
    , url : Endpoint
    , withCredentials : Bool
    }
    -> Http.Request a
request config =
    Http.request
        { body = config.body
        , expect = config.expect
        , headers = config.headers
        , method = config.method
        , timeout = config.timeout
        , url = unwrap config.url
        , withCredentials = config.withCredentials
        }



-- TYPES


{-| Get a URL to the stignore-agent API.

This is not publicly exposed, because we want to make sure the only way to get one of these URLs is from this module.

-}
type Endpoint
    = Endpoint String


unwrap : Endpoint -> String
unwrap (Endpoint str) =
    str


loadAgents : Endpoint
loadAgents =
    Url.Builder.absolute ["agents.json"] []
        |> Endpoint


url : List String -> List QueryParameter -> Endpoint
url paths queryParams =
    -- NOTE: Url.Builder takes care of percent-encoding special URL characters.
    -- See https://package.elm-lang.org/packages/elm/url/latest/Url#percentEncode
    Url.Builder.crossOrigin "https://localhost"
        ("api/v1" :: paths)
        queryParams
        |> Endpoint

agentUrl : Agent -> List String -> List QueryParameter -> Endpoint
agentUrl agent paths queryParams =
    Url.Builder.crossOrigin (Agents.host agent)
        ("api/v1" :: paths)
        queryParams
        |> Endpoint


-- AGENT ENDPOINTS


discover : Agent -> Endpoint
discover agent =
    agentUrl agent [ "discover" ] []


listing : Agent -> ContentType -> Endpoint
listing agent ctype =
    agentUrl agent [ ContentTypes.toString ctype, "listing" ] []


stignore : Agent -> ContentType -> Endpoint
stignore agent ctype =
    agentUrl agent [ ContentTypes.toString ctype, "stignore" ] []


flush : Agent -> ContentType -> Endpoint
flush agent ctype = 
    agentUrl agent [ ContentTypes.toString ctype, "stignore", "flush" ] []



-- ENDPOINTS


login : Endpoint
login =
    url [ "users", "login" ] []


user : Endpoint
user =
    url [ "user" ] []


users : Endpoint
users =
    url [ "users" ] []


follow : Username -> Endpoint
follow uname =
    url [ "profiles", Username.toString uname, "follow" ] []



-- ARTICLE ENDPOINTS


article : Slug -> Endpoint
article slug =
    url [ "articles", Slug.toString slug ] []


comments : Slug -> Endpoint
comments slug =
    url [ "articles", Slug.toString slug, "comments" ] []


comment : Slug -> CommentId -> Endpoint
comment slug commentId =
    url [ "articles", Slug.toString slug, "comments", CommentId.toString commentId ] []


favorite : Slug -> Endpoint
favorite slug =
    url [ "articles", Slug.toString slug, "favorite" ] []


articles : List QueryParameter -> Endpoint
articles params =
    url [ "articles" ] params


profiles : Username -> Endpoint
profiles uname =
    url [ "profiles", Username.toString uname ] []


feed : List QueryParameter -> Endpoint
feed params =
    url [ "articles", "feed" ] params


tags : Endpoint
tags =
    url [ "tags" ] []
