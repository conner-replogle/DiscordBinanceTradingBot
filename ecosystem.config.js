module.exports = {
  apps : [{
    name   : "TradingDiscordBot",
    script : ""
  }],
  deploy : {
    production : {
       "user" : "deployment",
       "host" : "192.46.229.57",
       "ref"  : "origin/production",
       "repo" : "git@github.com:conner-replogle/upwork_reda_shadi_rust.git",
       "path" : "/home/deployment/trading_bot",
       "post-deploy" : "cargo build"
    }
  }
}
