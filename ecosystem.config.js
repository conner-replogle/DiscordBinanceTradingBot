module.exports = {
  apps : [{
    name   : "TradingDiscordBot",
    script : ""
  }],
  deploy : {
    production : {
       "user" : "deployment",
       "host" : "192.46.229.57",
       "ref"  : "origin/master",
       "repo" : "git@github.com:Username/repository.git",
       "path" : "/var/www/my-repository",
       "post-deploy" : "npm install"
    }
  }
}
