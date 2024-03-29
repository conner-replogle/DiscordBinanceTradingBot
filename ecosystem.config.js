module.exports = {
  apps : [{
    name   : "TradingDiscordBot",
    script : "target/release/upwork_reda_shadi_rust"
  }],
  deploy : {
    production : {
       "user" : "deployment",
       "host" : "192.46.226.184",
       "ref"  : "origin/production",
       "repo" : "git@github.com:conner-replogle/upwork_reda_shadi_rust.git",
       "path" : "/home/deployment/trading_bot",
       "post-deploy" : "diesel migration run && cargo build --release && pm2 start ecosystem.config.js"
    }
  }
}
