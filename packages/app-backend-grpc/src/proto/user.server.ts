/* eslint-disable */
// @generated by protobuf-ts 2.7.0 with parameter eslint_disable,ts_nocheck,server_generic,client_none,optimize_code_size
// @generated from protobuf file "user.proto" (package "whiterabbit.user", syntax proto3)
// tslint:disable
// @ts-nocheck
import { Commands } from "./user";
import { Command } from "./user";
import { RpcInputStream } from "@protobuf-ts/runtime-rpc";
import { User } from "./user";
import { UserPage } from "./user";
import { FindPageRequest } from "./shared";
import { UserResponse } from "./user";
import { StringValue } from "./google/protobuf/wrappers";
import { ServerCallContext } from "@protobuf-ts/runtime-rpc";
/**
 * @generated from protobuf service whiterabbit.user.UserService
 */
export interface IUserService<T = ServerCallContext> {
  /**
   * @generated from protobuf rpc: findOne(google.protobuf.StringValue) returns (whiterabbit.user.UserResponse);
   */
  findOne(request: StringValue, context: T): Promise<UserResponse>;
  /**
   * @generated from protobuf rpc: findPage(whiterabbit.shared.FindPageRequest) returns (whiterabbit.user.UserPage);
   */
  findPage(request: FindPageRequest, context: T): Promise<UserPage>;
  /**
   * @generated from protobuf rpc: findAll(google.protobuf.StringValue) returns (stream whiterabbit.user.User);
   */
  findAll(
    request: StringValue,
    responses: RpcInputStream<User>,
    context: T
  ): Promise<void>;
  /**
   * @generated from protobuf rpc: handle(whiterabbit.user.Command) returns (whiterabbit.user.UserResponse);
   */
  handle(request: Command, context: T): Promise<UserResponse>;
  /**
   * @generated from protobuf rpc: handleAll(whiterabbit.user.Commands) returns (stream whiterabbit.user.UserResponse);
   */
  handleAll(
    request: Commands,
    responses: RpcInputStream<UserResponse>,
    context: T
  ): Promise<void>;
}
