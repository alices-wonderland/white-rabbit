import { Entity, Enum, ManyToOne, Unique } from "@mikro-orm/core";
import {
  AccessItemTypeValue,
  AccessItemAccessibleTypeValue,
} from "@white-rabbit/types";
import { UserEntity } from "../user";
import { GroupEntity } from "../group";
import { AbstractEntity } from "../shared";
// eslint-disable-next-line import/no-cycle
import JournalEntity from "./journal.entity";

@Entity({
  discriminatorColumn: "type",
  abstract: true,
  collection: "access_item",
})
@Unique({ properties: ["type", "accessible", "journal", "user", "group"] })
export default abstract class AccessItemValue extends AbstractEntity<AccessItemValue> {
  @Enum({ type: "string", items: () => AccessItemTypeValue })
  type: AccessItemTypeValue;

  @Enum({ type: "string", items: () => AccessItemAccessibleTypeValue })
  accessible: AccessItemAccessibleTypeValue;

  @ManyToOne(() => JournalEntity, {
    onDelete: "cascade",
    onUpdateIntegrity: "cascade",
  })
  journal: JournalEntity;

  abstract get itemId(): string;

  abstract get itemName(): string;

  abstract contains(user: string): Promise<boolean>;

  static create(
    items: Array<UserEntity | GroupEntity>,
    accessible: AccessItemAccessibleTypeValue
  ): AccessItemValue[] {
    return items.map((item) =>
      item instanceof UserEntity
        ? new AccessItemUserValue(item, accessible)
        : new AccessItemGroupValue(item, accessible)
    );
  }
}

@Entity({ discriminatorValue: AccessItemTypeValue.USER })
export class AccessItemUserValue extends AccessItemValue {
  @ManyToOne(() => UserEntity, {
    eager: true,
    onDelete: "cascade",
    onUpdateIntegrity: "cascade",
  })
  user: UserEntity;

  constructor(user: UserEntity, accessible: AccessItemAccessibleTypeValue) {
    super();
    this.type = AccessItemTypeValue.USER;
    this.accessible = accessible;
    this.user = user;
  }

  get itemId(): string {
    return this.user.id;
  }

  get itemName(): string {
    return this.user.name;
  }

  // Only non-deleted user can be inited
  async contains(userId: string): Promise<boolean> {
    return this.user.isInitialized() && userId === this.user.id;
  }
}

@Entity({ discriminatorValue: AccessItemTypeValue.GROUP })
export class AccessItemGroupValue extends AccessItemValue {
  @ManyToOne(() => GroupEntity, {
    eager: true,
    onDelete: "cascade",
    onUpdateIntegrity: "cascade",
  })
  group: GroupEntity;

  constructor(group: GroupEntity, accessible: AccessItemAccessibleTypeValue) {
    super();
    this.type = AccessItemTypeValue.GROUP;
    this.accessible = accessible;
    this.group = group;
  }

  get itemId(): string {
    return this.group.id;
  }

  get itemName(): string {
    return this.group.name;
  }

  // Only non-deleted group can be inited
  async contains(userId: string): Promise<boolean> {
    return this.group.isInitialized() && this.group.contains(userId);
  }
}
