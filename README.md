# matrix-wechat
A Matrix-WeChat puppeting bridge implemented in Rust.

### Documentation

Some quick links:

* [Agent setup](https://github.com/duo/matrix-wechat-agent)
* [Docker](https://hub.docker.com/r/lxduo/matrix-wechat)
* [Step by Step (Chinese)](https://duo.github.io/posts/matrix-qq-wechat/)

### Features & roadmap

* Matrix → WeChat
  * [ ] Message types
    * [x] Text
	* [x] Image
	* [x] Sticker
	* [x] Video
	* [ ] Audio
    * [x] File
    * [x] Mention
    * [ ] Reply
    * [ ] Location
  * [x] Chat types
	* [x] Direct
	* [x] Room
  * [ ] Presence
  * [ ] Redaction
  * [ ] Group actions
    * [ ] Join
    * [ ] Invite
    * [ ] Leave
    * [ ] Kick
	* [ ] Mute
  * [ ] Room metadata
    * [ ] Name
    * [ ] Avatar
    * [ ] Topic
  * [ ] User metadata
    * [ ] Name
    * [ ] Avatar

* WeChat → Matrix
  * [ ] Message types
    * [x] Text
    * [x] Image
    * [x] Sticker
    * [x] Video
    * [x] Audio
    * [x] File
    * [x] Mention
    * [x] Reply
    * [x] Location
  * [ ] Chat types
    * [x] Private
    * [x] Group
  * [ ] Presence
  * [x] Redaction
  * [ ] Group actions
    * [ ] Invite
    * [ ] Join
    * [ ] Leave
    * [ ] Kick
	* [ ] Mute
  * [ ] Group metadata
    * [x] Name
    * [x] Avatar
	* [x] Topic
  * [x] User metadata
    * [x] Name
    * [x] Avatar
  * [ ] Login types
	* [ ] Password
	* [x] QR code

* Misc
  * [ ] Automatic portal creation
    * [ ] After login
    * [ ] When added to group
    * [x] When receiving message
  * [x] Double puppeting

## Palpo KDL Configuration

When running in the [Palpo](https://github.com/palpo-im/palpo) environment, you can use KDL format for configuration. See [`config.example.kdl`](config.example.kdl) for the full annotated example.

Key sections in KDL format:

```kdl
homeserver {
    address "https://matrix.example.com"
    domain "example.com"
    software "standard"
}

appservice {
    address "http://localhost:17778"
    hostname "0.0.0.0"
    port 17778
    database {
        type "postgres"
        uri "postgres://user:password@host/database?sslmode=disable"
    }
    id "wechat"
    bot {
        username "wechatbot"
        displayname "WeChat bridge bot"
    }
    as_token "replace-with-as-token"
    hs_token "replace-with-hs-token"
}

bridge {
    username_template "_wechat_{{.}}"
    displayname_template "{{if .Name}}{{.Name}}{{else}}{{.Uin}}{{end}} (WeChat)"
    listen_address "0.0.0.0:20002"
    command_prefix "!wechat"
    encryption {
        allow false
        default false
    }
    permissions {
        "example.com" "user"
        "@admin:example.com" "admin"
    }
}

logging {
    min_level "debug"
    writers {
        - type="stdout" format="pretty-colored"
    }
}
```
