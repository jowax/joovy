import * as dotenv from 'dotenv'

export type Config = ReturnType<typeof initConfig>

export function initConfig() {
  // init config from .env
  dotenv.config()

  return {
    token: process.env.TOKEN ?? throwError('No token defined...'),
    applicationId: process.env.APPLICATION_ID ?? throwError('Application id is not defined...'),
    testGuildId: process.env.TEST_GUILD_ID,
  }
}

const throwError = (error: string) => {
  throw Error(error)
}
