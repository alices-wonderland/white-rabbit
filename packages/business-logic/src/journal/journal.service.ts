import { inject, singleton } from "tsyringe";
import { EntityManager, MikroORM, ObjectQuery } from "@mikro-orm/core";
import {
  AccessItemTypeValue,
  AdditionalQuery,
  CONTAINING_USER_OPERATOR,
  CreateJournalCommand,
  DeleteJournalCommand,
  FULL_TEXT_OPERATOR,
  JOURNAL_READ_SCOPE,
  JOURNAL_WRITE_SCOPE,
  JournalCommand,
  JournalQuery,
  RoleValue,
  UpdateJournalCommand,
  AccessItemAccessibleTypeValue,
} from "@white-rabbit/types";
import isEmpty from "lodash/isEmpty";
import { AuthUser, checkCreate, CommandInput, WriteService } from "../shared";
import { UserEntity } from "../user";
import { AlreadyArchivedError, NoPermissionError } from "../error";
import { accessItemsContain, filterAsync, fullTextSearch } from "../utils";
import JournalEntity, { JOURNAL_TYPE } from "./journal.entity";
import AccessItemService from "./access-item.service";
import { AccessItemValue } from "./index";

@singleton()
export default class JournalService extends WriteService<
  JournalEntity,
  JournalCommand,
  JournalQuery
> {
  constructor(
    @inject(MikroORM) readonly orm: MikroORM,
    @inject(AccessItemService)
    private readonly accessItemService: AccessItemService
  ) {
    super(
      orm,
      JOURNAL_TYPE,
      JournalEntity,
      JOURNAL_READ_SCOPE,
      JOURNAL_WRITE_SCOPE,
      ["CreateJournalCommand"]
    );
  }

  private async createJournal(
    authUser: AuthUser,
    command: CreateJournalCommand,
    em: EntityManager
  ): Promise<JournalEntity> {
    await checkCreate(
      this.type,
      this.entityName,
      authUser,
      this.writeScope,
      {
        name: command.name,
      },
      em
    );

    const admins = await this.accessItemService.loadAll(
      authUser,
      command.admins,
      em
    );
    const members = await this.accessItemService.loadAll(
      authUser,
      command.members,
      em
    );
    const entity = new JournalEntity(
      command.name,
      command.description,
      command.tags,
      command.unit
    );
    entity.setAccessItems(admins, AccessItemAccessibleTypeValue.ADMIN);
    entity.setAccessItems(members, AccessItemAccessibleTypeValue.MEMBER);
    em.persist(entity);
    return entity;
  }

  private async updateJourney(
    authUser: AuthUser,
    command: UpdateJournalCommand,
    em: EntityManager
  ): Promise<JournalEntity> {
    const entity = await this.getWriteableEntity(
      authUser,
      command.targetId,
      em
    );

    if (
      command.name == null &&
      command.description == null &&
      command.tags == null &&
      command.unit == null &&
      command.admins == null &&
      command.members == null
    ) {
      return entity;
    }

    if (command.name != null) {
      entity.name = command.name;
    }

    if (command.description != null) {
      entity.description = command.description;
    }

    if (command.tags != null) {
      entity.tags = command.tags;
    }

    if (command.unit != null) {
      entity.unit = command.unit;
    }

    if (command.admins != null) {
      const admins = await this.accessItemService.loadAll(
        authUser,
        command.admins,
        em
      );
      entity.setAccessItems(admins, AccessItemAccessibleTypeValue.ADMIN);
    }

    if (command.members != null) {
      const members = await this.accessItemService.loadAll(
        authUser,
        command.members,
        em
      );
      entity.setAccessItems(members, AccessItemAccessibleTypeValue.MEMBER);
    }

    em.persist(entity);
    return entity;
  }

  private async deleteJournal(
    authUser: AuthUser,
    command: DeleteJournalCommand,
    em: EntityManager
  ): Promise<void> {
    const entity = await this.getWriteableEntity(
      authUser,
      command.targetId,
      em
    );

    em.remove(entity);
  }

  async doHandle(
    { authUser, command }: CommandInput<JournalCommand>,
    em: EntityManager
  ): Promise<JournalEntity | null> {
    switch (command.type) {
      case "CreateJournalCommand":
        return this.createJournal(authUser, command, em);
      case "UpdateJournalCommand":
        return this.updateJourney(authUser, command, em);
      case "DeleteJournalCommand":
        return this.deleteJournal(authUser, command, em).then(() => null);
    }
  }

  async isAdmin(entity: JournalEntity, user: UserEntity): Promise<boolean> {
    if (!entity.accessItems.isInitialized()) {
      await entity.accessItems.init();
    }

    const adminContains = await Promise.all(
      entity.admins.map(async (item) => item.contains(user.id))
    );

    return adminContains.some((isAdmin) => isAdmin);
  }

  override async isReadable(
    entity: JournalEntity,
    { user }: AuthUser
  ): Promise<boolean> {
    if ((user?.role ?? RoleValue.USER) !== RoleValue.USER) {
      return true;
    }

    if (!entity.accessItems.isInitialized()) {
      await entity.accessItems.init();
    }

    const accessItemsContains = await Promise.all(
      entity.accessItems
        .getItems()
        .map(async (item) => item.contains(user?.id ?? ""))
    );

    return !accessItemsContains.every((contains) => !contains);
  }

  async checkWriteable(
    entity: JournalEntity,
    authUser: AuthUser
  ): Promise<void> {
    await super.checkWriteable(entity, authUser);

    const user = authUser.user;

    if (user == null) {
      throw new NoPermissionError(this.type, "WRITE");
    }

    if (user.role !== RoleValue.USER) {
      return;
    }

    if (entity.archived) {
      throw new AlreadyArchivedError(this.type, entity.id);
    }

    if (!(await this.isAdmin(entity, user))) {
      throw new NoPermissionError(this.type, "WRITE");
    }
  }

  // eslint-disable-next-line sonarjs/cognitive-complexity
  async handleAdditionalQuery(
    authUser: AuthUser,
    entities: JournalEntity[],
    query: AdditionalQuery
  ): Promise<JournalEntity[]> {
    if (query.type === "ContainingUserQuery") {
      return filterAsync(entities, async (entity) => {
        for (const field of query.fields) {
          const value = entity[field as keyof JournalEntity];
          if (
            value instanceof Array &&
            (await accessItemsContain(value as AccessItemValue[], query.user))
          ) {
            return true;
          }
        }
        return false;
      });
    } else if (query.type === "FullTextQuery") {
      return filterAsync(entities, async (entity) =>
        fullTextSearch(entity, query)
      );
    } else {
      return super.handleAdditionalQuery(authUser, entities, query);
    }
  }

  // eslint-disable-next-line sonarjs/cognitive-complexity
  override doGetQueries(
    query: JournalQuery
  ): [AdditionalQuery[], ObjectQuery<JournalEntity>] {
    const additionalQuery: AdditionalQuery[] = [];
    const objectQuery: ObjectQuery<JournalEntity> = {};

    if (query.includeArchived !== true) {
      objectQuery.archived = false;
    }

    for (const [key, value] of Object.entries(query)) {
      if (key === FULL_TEXT_OPERATOR && !isEmpty(value)) {
        additionalQuery.push({
          type: "FullTextQuery",
          value,
          fields: ["name", "description", "tags"],
        });
      } else if (key === CONTAINING_USER_OPERATOR && !isEmpty(value)) {
        additionalQuery.push({
          type: "ContainingUserQuery",
          user: value,
          fields: ["admins", "members"],
        });
      } else if (key === "id" && !isEmpty(value)) {
        objectQuery.id = value;
      } else if (key === "name") {
        if (typeof value === "string" && !isEmpty(value)) {
          objectQuery.name = value;
        } else if (
          FULL_TEXT_OPERATOR in value &&
          !isEmpty(value[FULL_TEXT_OPERATOR])
        ) {
          additionalQuery.push({
            type: "FullTextQuery",
            value: value[FULL_TEXT_OPERATOR],
            fields: ["name"],
          });
        }
      } else if (key === "description" && !isEmpty(value[FULL_TEXT_OPERATOR])) {
        additionalQuery.push({
          type: "FullTextQuery",
          value: value[FULL_TEXT_OPERATOR],
          fields: ["description"],
        });
      } else if (key === "tags") {
        if (
          FULL_TEXT_OPERATOR in value &&
          !isEmpty(value[FULL_TEXT_OPERATOR])
        ) {
          additionalQuery.push({
            type: "FullTextQuery",
            value: value[FULL_TEXT_OPERATOR],
            fields: ["tags"],
          });
        } else if (
          (typeof value === "string" || value instanceof Array) &&
          !isEmpty(value)
        ) {
          objectQuery.tags = value;
        }
      } else if (key === "unit" && !isEmpty(value)) {
        objectQuery.unit = value;
      } else if (key === "admins") {
        objectQuery.accessItems = {
          accessible: AccessItemAccessibleTypeValue.ADMIN,
          type: value.type,
          [value.type === AccessItemTypeValue.USER ? "user" : "group"]:
            value.id,
        };
      } else if (key === "members") {
        objectQuery.accessItems = {
          accessible: AccessItemAccessibleTypeValue.MEMBER,
          type: value.type,
          [value.type === AccessItemTypeValue.USER ? "user" : "group"]:
            value.id,
        };
      }
    }

    return [additionalQuery, objectQuery];
  }
}
