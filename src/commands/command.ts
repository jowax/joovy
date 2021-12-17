import { Observable } from 'rxjs'
import JEvent, { ResultEntry } from '../jevent/JEvent'
import ArgParser from './ArgParser'
import Disconnect from './impl/Disconnect'
import Help from './impl/Help'
import Play from './impl/Play'
import Queue from './impl/Queue'

export default interface Command {
  /**
   * Defines when the current command should be run, its arguments and a description of it.
   */
  argument: ArgParser

  /**
   * Describes what the command does.
   */
  helpText: string

  /**
   * Will be called if the message sent matches the one defined in argument.
   */
  handleMessage(event: JEvent): Observable<ResultEntry>
}

const cmds = [
  new Play(),
  new Disconnect(),
  new Queue(),
]

const help = new Help(cmds)

export const handle = (event: JEvent): Observable<ResultEntry> => {
  const content = event.message.content

  for (const cmd of cmds) {
    if (cmd.argument.is(content)) {
      return event.result(
        { commandCalled: cmd.argument.command },
        cmd.handleMessage(event),
      )
    }
  }

  return event.result(
    { invalidCommand: content },
    help.handleMessage(event),
  )
}
