import { AdditionalQuery } from "@white-rabbit/types";
import AbstractError from "./abstract-error";

export default class InvalidQueryError extends AbstractError {
  protected readonly type: string = "InvalidQueryError";

  constructor(query: AdditionalQuery["type"], entityType: string) {
    super(`Query[${query}] is invalid for Type[${entityType}]`);
  }
}
