package dev.oreslang.compiler;

import dev.oreslang.ast.Ast;
import dev.oreslang.imports.ImportRules;

import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * AOT-friendly linked-unit check for direct imported module operations.
 * Reads, calls and writes are checked against exported declarations rather
 * than treating unknown imported namespace values as unrestricted dynamic data.
 *
 * No reflection, runtime class loading or global mutable registry is used.
 */
final class LinkedModuleContractChecker {
    private final String importer;
    private final Map<String, Ast.ModuleDecl> aliases = new HashMap<>();

    private LinkedModuleContractChecker(String importer) { this.importer = importer; }

    static void check(Map<String, Ast.Program> units,
            Map<String, Map<String, String>> resolutions) {
        for (Map.Entry<String, Ast.Program> file : units.entrySet()) {
            LinkedModuleContractChecker checker = new LinkedModuleContractChecker(file.getKey());
            for (Ast.ImportDecl imp : file.getValue().imports()) {
                if (imp.kind() != Ast.ImportKind.MODULE || imp.wildcard()) continue;
                String targetId = ImportGraph.resolveImportUnitId(
                        file.getKey(), imp, units.keySet(), resolutions);
                if (targetId == null) continue; // external package resolver
                Ast.Program target = units.get(targetId);
                for (String exported : imp.names()) {
                    Ast.ModuleDecl module = null;
                    for (Ast.ModuleDecl candidate : target.modules()) {
                        if (candidate.name().equals(exported)) module = candidate;
                    }
                    if (module == null) throw new IllegalArgumentException(
                            "missing import module " + exported + " from " + targetId);
                    checker.aliases.put(ImportRules.localName(imp, exported), module);
                }
            }
            if (checker.aliases.isEmpty()) continue;
            for (Ast.ModuleDecl module : file.getValue().modules()) {
                for (Ast.Decl decl : module.declarations()) {
                    if (decl instanceof Ast.FunctionDecl fn) {
                        checker.body(fn.body(), checker.parameters(fn.parameters()));
                    } else if (decl instanceof Ast.FieldDecl field) {
                        checker.expr(field.initializer(), Set.of());
                    } else if (decl instanceof Ast.ClassDecl klass) {
                        for (Ast.FieldDecl field : klass.fields()) {
                            checker.expr(field.initializer(), Set.of());
                        }
                        if (klass.constructor() != null) checker.body(
                                klass.constructor().body(),
                                checker.parameters(klass.constructor().parameters()));
                        for (Ast.MethodDecl method : klass.methods()) {
                            checker.body(method.body(), checker.parameters(method.parameters()));
                        }
                    }
                }
            }
        }
    }

    private Set<String> parameters(List<Ast.Param> params) {
        Set<String> names = new HashSet<>();
        for (Ast.Param param : params) names.add(param.name());
        return names;
    }

    private Ast.ModuleDecl namespace(Ast.Expr expression, Set<String> locals) {
        if (expression instanceof Ast.NameExpr name && !locals.contains(name.name())) {
            return aliases.get(name.name());
        }
        return null;
    }

    private Ast.Decl requireExport(Ast.ModuleDecl module, String member) {
        Ast.Decl found = null;
        for (Ast.Decl decl : module.declarations()) {
            if (decl instanceof Ast.FunctionDecl fn
                    && fn.name().equals(member)
                    && fn.visibility() == Ast.Visibility.PUBLIC) {
                if (found != null && !(found instanceof Ast.FunctionDecl)) {
                    throw new IllegalArgumentException("ambiguous imported member " + member);
                }
                found = decl;
            } else if (decl instanceof Ast.FieldDecl field
                    && field.name().equals(member)
                    && field.visibility() == Ast.Visibility.PUBLIC) {
                if (found != null) throw new IllegalArgumentException(
                        "ambiguous imported member " + member);
                found = decl;
            } else if (decl instanceof Ast.ClassDecl klass
                    && klass.name().equals(member)
                    && klass.visibility() == Ast.Visibility.PUBLIC) {
                if (found != null) throw new IllegalArgumentException(
                        "ambiguous imported member " + member);
                found = decl;
            }
        }
        if (found == null) throw new IllegalArgumentException(
                "importer '" + importer + "' accesses non-public or missing module member "
                        + module.name() + "." + member);
        return found;
    }

    private void checkCall(Ast.CallExpr call, Set<String> locals) {
        if (!(call.callee() instanceof Ast.MemberExpr member)) return;
        Ast.ModuleDecl module = namespace(member.receiver(), locals);
        if (module == null) return;
        int matches = 0;
        Ast.FunctionDecl selected = null;
        for (Ast.Decl decl : module.declarations()) {
            if (decl instanceof Ast.FunctionDecl fn
                    && fn.name().equals(member.member())
                    && fn.visibility() == Ast.Visibility.PUBLIC
                    && fn.parameters().size() == call.arguments().size()
                    && fn.actorKind() == Ast.ActorKind.NONE) {
                matches++;
                selected = fn;
            }
        }
        if (matches != 1) throw new IllegalArgumentException(
                "imported module callable " + module.name() + "." + member.member()
                        + " requires exactly one public non-actor overload of arity "
                        + call.arguments().size());
        for (int i = 0; i < selected.parameters().size(); i++) {
            checkLiteralContract(call.arguments().get(i),
                    selected.parameters().get(i).type(),
                    module.name() + "." + member.member() + " argument " + (i + 1));
        }
    }

    private static void checkLiteralContract(Ast.Expr actual, Ast.TypeRef expected, String where) {
        if (!(actual instanceof Ast.LiteralExpr literal) || expected == null) return;
        Object value = literal.value();
        String name = expected.name();
        boolean mismatch = switch (name) {
            case "int", "uint", "i8", "i16", "i32", "i64",
                    "u8", "u16", "u32", "u64", "bigint" ->
                    !(value instanceof Byte || value instanceof Short
                            || value instanceof Integer || value instanceof Long
                            || value instanceof java.math.BigInteger);
            case "float", "f32", "f64", "decimal" -> !(value instanceof Number);
            case "string", "String" -> !(value instanceof String);
            case "bool", "Bool" -> !(value instanceof Boolean);
            default -> false; // symbolic/generic types require full typed linker
        };
        if (mismatch) throw new IllegalArgumentException(
                "cross-file module ABI type mismatch at " + where
                        + ": expected " + expected.name());
    }

    private void checkWrite(Ast.Expr target, Set<String> locals) {
        if (!(target instanceof Ast.MemberExpr member)) return;
        Ast.ModuleDecl module = namespace(member.receiver(), locals);
        if (module == null) return;
        Ast.Decl decl = requireExport(module, member.member());
        if (!(decl instanceof Ast.FieldDecl field)
                || field.bindingKind() != Ast.BindingKind.LET) {
            throw new IllegalArgumentException(
                    "imported module field " + module.name() + "." + member.member()
                            + " is not a public mutable let");
        }
    }

    private void body(List<Ast.Stmt> statements, Set<String> inherited) {
        Set<String> locals = new HashSet<>(inherited);
        for (Ast.Stmt stmt : statements) {
            if (stmt instanceof Ast.BindingStmt s) {
                expr(s.initializer(), locals);
                locals.add(s.name());
            } else if (stmt instanceof Ast.DestructureStmt s) {
                expr(s.initializer(), locals);
                for (Ast.DestructureBinding binding : s.bindings()) {
                    if (!binding.isDiscard()) locals.add(binding.name());
                }
            } else if (stmt instanceof Ast.ExprStmt s) expr(s.expression(), locals);
            else if (stmt instanceof Ast.ReturnStmt s) expr(s.value(), locals);
            else if (stmt instanceof Ast.YieldStmt s) expr(s.value(), locals);
            else if (stmt instanceof Ast.DeferStmt s) expr(s.expression(), locals);
            else if (stmt instanceof Ast.BlockStmt s) body(s.body(), locals);
            else if (stmt instanceof Ast.IfStmt s) {
                for (Ast.IfBranch branch : s.branches()) {
                    expr(branch.condition(), locals);
                    body(branch.body(), locals);
                }
                body(s.elseBody(), locals);
            } else if (stmt instanceof Ast.MatchStmt s) {
                expr(s.subject(), locals);
                for (Ast.MatchArm arm : s.arms()) {
                    expr(arm.guard(), locals);
                    body(arm.body(), locals);
                }
            } else if (stmt instanceof Ast.SwitchStmt s) {
                expr(s.subject(), locals);
                for (Ast.SwitchCase branch : s.cases()) {
                    for (Ast.Expr constant : branch.constants()) expr(constant, locals);
                    body(branch.body(), locals);
                }
                body(s.defaultBody(), locals);
            } else if (stmt instanceof Ast.TryStmt s) {
                body(s.body(), locals);
                Set<String> catchScope = new HashSet<>(locals);
                if (s.errorName() != null) catchScope.add(s.errorName());
                body(s.catchBody(), catchScope);
                body(s.finallyBody(), locals);
            } else if (stmt instanceof Ast.ForOfStmt s) {
                expr(s.iterable(), locals);
                Set<String> scope = new HashSet<>(locals);
                scope.add(s.bindingName());
                body(s.body(), scope);
            } else if (stmt instanceof Ast.ForOfDestructureStmt s) {
                expr(s.iterable(), locals);
                Set<String> scope = new HashSet<>(locals);
                for (Ast.DestructureBinding binding : s.bindings()) {
                    if (!binding.isDiscard()) scope.add(binding.name());
                }
                body(s.body(), scope);
            } else if (stmt instanceof Ast.ForStmt s) {
                if (s.initializer() != null) body(List.of(s.initializer()), locals);
                expr(s.condition(), locals);
                expr(s.update(), locals);
                body(s.body(), locals);
            } else if (stmt instanceof Ast.LoopStmt s) body(s.body(), locals);
            else if (stmt instanceof Ast.SelectStmt s) {
                for (Ast.SelectArm arm : s.arms()) {
                    expr(arm.channel(), locals);
                    expr(arm.value(), locals);
                    Set<String> scope = new HashSet<>(locals);
                    if (arm.bindingName() != null) scope.add(arm.bindingName());
                    body(arm.body(), scope);
                }
            }
        }
    }

    private void expr(Ast.Expr e, Set<String> locals) {
        if (e == null) return;
        if (e instanceof Ast.MemberExpr member) {
            Ast.ModuleDecl owner = namespace(member.receiver(), locals);
            if (owner != null) requireExport(owner, member.member());
            expr(member.receiver(), locals);
        } else if (e instanceof Ast.AssignExpr s) {
            checkWrite(s.target(), locals);
            expr(s.target(), locals);
            expr(s.value(), locals);
        } else if (e instanceof Ast.CallExpr s) {
            checkCall(s, locals);
            expr(s.callee(), locals);
            for (Ast.Expr argument : s.arguments()) expr(argument, locals);
        } else if (e instanceof Ast.BinaryExpr s) {
            expr(s.left(), locals);
            expr(s.right(), locals);
        } else if (e instanceof Ast.UnaryExpr s) expr(s.operand(), locals);
        else if (e instanceof Ast.ConditionalExpr s) {
            expr(s.condition(), locals);
            expr(s.whenTrue(), locals);
            expr(s.whenFalse(), locals);
        } else if (e instanceof Ast.TypeTestExpr s) expr(s.value(), locals);
        else if (e instanceof Ast.PatternTestExpr s) expr(s.value(), locals);
        else if (e instanceof Ast.CastExpr s) expr(s.value(), locals);
        else if (e instanceof Ast.SpreadExpr s) expr(s.expression(), locals);
        else if (e instanceof Ast.RuntimeCallExpr s) {
            for (Ast.Expr argument : s.arguments()) expr(argument, locals);
        } else if (e instanceof Ast.IndexExpr s) {
            if (namespace(s.receiver(), locals) != null) {
                throw new IllegalArgumentException(
                        "computed indexing of an imported module namespace is forbidden");
            }
            expr(s.receiver(), locals);
            expr(s.index(), locals);
        } else if (e instanceof Ast.NewExpr s) {
            for (Ast.Expr argument : s.arguments()) expr(argument, locals);
        } else if (e instanceof Ast.AwaitExpr s) expr(s.expression(), locals);
        else if (e instanceof Ast.ChannelOpExpr s) {
            expr(s.channel(), locals);
            expr(s.value(), locals);
            body(s.callbackBody(), locals);
        } else if (e instanceof Ast.DynamicSelectExpr s) expr(s.cases(), locals);
        else if (e instanceof Ast.ListExpr s) {
            for (Ast.Expr element : s.elements()) expr(element, locals);
        } else if (e instanceof Ast.TupleExpr s) {
            for (Ast.Expr element : s.elements()) expr(element, locals);
        } else if (e instanceof Ast.ObjectExpr s) {
            for (Ast.ObjectField field : s.fields()) expr(field.value(), locals);
        } else if (e instanceof Ast.LambdaExpr s) {
            Set<String> scope = new HashSet<>(locals);
            for (Ast.Param param : s.parameters()) scope.add(param.name());
            expr(s.expressionBody(), scope);
            if (s.blockBody() != null) body(s.blockBody(), scope);
        }
    }
}
