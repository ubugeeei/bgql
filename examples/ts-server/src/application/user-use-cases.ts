/**
 * User Use Cases
 *
 * Application-level business logic for user operations.
 * Uses Result types for railway-oriented error handling.
 */

import { Result, ok, err } from "@bgql/client";
import {
  User, UserId, Email, UserRole, UserAnalytics,
} from "../domain/entities.js";
import {
  DomainError,
  UserNotFoundError,
  UniqueConstraintError,
  ValidationError,
  UnauthorizedError,
  EmailFormatError,
  MinLengthError,
} from "../domain/errors.js";
import {
  UserRepository,
  CreateUserData,
  UpdateUserData,
  UserFilter,
} from "../infrastructure/repositories.js";

// ============================================
// Input Validation
// ============================================

function validateEmail(email: string): Result<Email, ValidationError> {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(email)) {
    return err(new EmailFormatError());
  }
  return ok(Email(email));
}

function validateName(name: string): Result<string, ValidationError> {
  const trimmed = name.trim();
  if (trimmed.length < 2) {
    return err(new MinLengthError("name", 2));
  }
  return ok(trimmed);
}

// ============================================
// Query Use Cases
// ============================================

export interface GetUserQuery {
  readonly id: UserId;
}

export interface GetUserByEmailQuery {
  readonly email: string;
}

export interface ListUsersQuery {
  readonly filter?: UserFilter;
}

export class UserQueryService {
  constructor(private readonly userRepo: UserRepository) {}

  async getUser(query: GetUserQuery): Promise<Result<User, UserNotFoundError>> {
    return this.userRepo.findById(query.id);
  }

  async getUserByEmail(query: GetUserByEmailQuery): Promise<Result<User, UserNotFoundError | ValidationError>> {
    const emailResult = validateEmail(query.email);
    if (!emailResult.ok) {
      return err(emailResult.error);
    }
    return this.userRepo.findByEmail(emailResult.value);
  }

  async listUsers(query: ListUsersQuery): Promise<User[]> {
    return this.userRepo.findAll(query.filter);
  }

  async getUserAnalytics(id: UserId): Promise<UserAnalytics> {
    return this.userRepo.getAnalytics(id);
  }
}

// ============================================
// Command Use Cases
// ============================================

export interface CreateUserCommand {
  readonly name: string;
  readonly email: string;
  readonly bio?: string | null;
  readonly role?: UserRole;
}

export interface UpdateUserCommand {
  readonly id: UserId;
  readonly name?: string;
  readonly email?: string;
  readonly bio?: string | null;
  readonly avatarUrl?: string | null;
}

export interface DeleteUserCommand {
  readonly id: UserId;
  readonly requesterId: UserId;
  readonly requesterRole: UserRole;
}

export class UserCommandService {
  constructor(private readonly userRepo: UserRepository) {}

  async createUser(command: CreateUserCommand): Promise<Result<User, ValidationError | UniqueConstraintError>> {
    // Validate name
    const nameResult = validateName(command.name);
    if (!nameResult.ok) {
      return err(nameResult.error);
    }

    // Validate email
    const emailResult = validateEmail(command.email);
    if (!emailResult.ok) {
      return err(emailResult.error);
    }

    const data: CreateUserData = {
      name: nameResult.value,
      email: emailResult.value,
      bio: command.bio,
      role: command.role,
    };

    return this.userRepo.create(data);
  }

  async updateUser(command: UpdateUserCommand): Promise<Result<User, UserNotFoundError | ValidationError>> {
    // Validate name if provided
    let validatedName: string | undefined;
    if (command.name !== undefined) {
      const nameResult = validateName(command.name);
      if (!nameResult.ok) {
        return err(nameResult.error);
      }
      validatedName = nameResult.value;
    }

    // Validate email if provided
    let validatedEmail: Email | undefined;
    if (command.email !== undefined) {
      const emailResult = validateEmail(command.email);
      if (!emailResult.ok) {
        return err(emailResult.error);
      }
      validatedEmail = emailResult.value;
    }

    const data: UpdateUserData = {
      name: validatedName,
      email: validatedEmail,
      bio: command.bio,
      avatarUrl: command.avatarUrl,
    };

    return this.userRepo.update(command.id, data);
  }

  async deleteUser(command: DeleteUserCommand): Promise<Result<void, UserNotFoundError | UnauthorizedError>> {
    // Authorization check: only Admin or the user themselves can delete
    if (command.requesterRole !== "Admin" && command.requesterId !== command.id) {
      return err(new UnauthorizedError("Only admins can delete other users", "Admin"));
    }

    return this.userRepo.delete(command.id);
  }
}
