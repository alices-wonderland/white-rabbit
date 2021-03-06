import { AbstractEntity, FindInput } from "@white-rabbit/business-logic";
import AbstractExceptionTask from "./abstract-exception-task";

export default interface FindOneExceptionTask<E extends AbstractEntity<E>, Q>
  extends AbstractExceptionTask<E, FindInput<E, Q>, E | null> {
  readonly type: "FindOneExceptionTask";
}
