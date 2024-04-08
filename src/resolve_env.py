import asv
import json

conf = asv.config.Config.load("asv.conf.json")
env_names = [env.name for env in asv.environment.get_environments(conf, "")]
print(json.dumps(env_names))