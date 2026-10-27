//! Compile complete clipping chains, including backdrop-dependent pass-through.
use super::*;

struct Compiler<'a> {
    nodes: &'a [Node],
    children: Vec<Vec<usize>>,
    operations: Vec<Operation>,
    extras: u32,
    surfaces: u32,
}
impl Compiler<'_> {
    fn emit(
        &mut self,
        kind: OperationKind,
        node: usize,
        output: u32,
        source: u32,
        backdrop: u32,
        coverage: Option<u32>,
    ) -> Result<(), PlanError> {
        if self.operations.len() > self.nodes.len() * 16 {
            return Err(PlanError::Budget);
        }
        self.surfaces = self.surfaces.max(output + 1);
        self.operations.push(Operation {
            kind,
            node: node as u32,
            output,
            source,
            backdrop,
            coverage,
        });
        Ok(())
    }
    fn group(&mut self, i: usize, depth: u32, before: Option<u32>) -> Result<u32, PlanError> {
        let start = (depth + 1) * 2;
        self.emit(
            if before.is_some() {
                OperationKind::Copy
            } else {
                OperationKind::Clear
            },
            i,
            start,
            before.unwrap_or(0),
            before.unwrap_or(0),
            None,
        )?;
        self.stack(i, depth + 1, start)
    }
    fn raw_masked(&mut self, i: usize, depth: u32, output: u32) -> Result<(), PlanError> {
        if self.nodes[i].kind == NodeKind::Group {
            let source = self.group(i, depth, None)?;
            self.emit(OperationKind::MaskSurface, i, output, source, source, None)
        } else {
            self.emit(OperationKind::MaskedRaster, i, output, 0, 0, None)
        }
    }
    fn clip(
        &mut self,
        i: usize,
        depth: u32,
        before: u32,
        output: u32,
        coverage: Option<u32>,
    ) -> Result<(), PlanError> {
        if self.nodes[i].kind == NodeKind::Group {
            let through = self.nodes[i].blend == BlendMode::PassThrough;
            let source = self.group(i, depth, if through { Some(before) } else { None })?;
            self.emit(
                if through {
                    OperationKind::ClipInfluence
                } else {
                    OperationKind::ClipSurface
                },
                i,
                output,
                source,
                before,
                coverage,
            )
        } else {
            self.emit(
                OperationKind::ClipRaster,
                i,
                output,
                before,
                before,
                coverage,
            )
        }
    }
    fn stack(&mut self, parent: usize, depth: u32, mut current: u32) -> Result<u32, PlanError> {
        if depth > 16 {
            return Err(PlanError::Budget);
        }
        let first = depth * 2;
        let extra = self.extras + depth * 3;
        let mut at = 0;
        while at < self.children[parent].len() {
            let i = self.children[parent][at];
            let mut end = at + 1;
            while end < self.children[parent].len()
                && self.nodes[self.children[parent][end]].clipped
            {
                end += 1;
            }
            if self.nodes[i].clipped || !self.nodes[i].visible {
                at = end;
                continue;
            }
            let output = first + u32::from(current == first);
            let through = self.nodes[i].blend == BlendMode::PassThrough;
            if end == at + 1 {
                if self.nodes[i].kind == NodeKind::Group {
                    let source =
                        self.group(i, depth, if through { Some(current) } else { None })?;
                    self.emit(
                        if through {
                            OperationKind::Influence
                        } else {
                            OperationKind::BlendSurface
                        },
                        i,
                        output,
                        source,
                        current,
                        None,
                    )?;
                } else {
                    self.emit(OperationKind::Raster, i, output, current, current, None)?;
                }
            } else {
                if through {
                    let source = self.group(i, depth, Some(current))?;
                    self.emit(OperationKind::Copy, i, extra + 1, source, source, None)?;
                    let intrinsic = self.group(i, depth, None)?;
                    self.emit(OperationKind::Copy, i, extra, intrinsic, intrinsic, None)?;
                } else {
                    self.raw_masked(i, depth, extra + 1)?;
                }
                let mut source = extra + 1;
                for n in at + 1..end {
                    let c = self.children[parent][n];
                    if !self.nodes[c].visible {
                        continue;
                    }
                    let next = extra + if source == extra + 1 { 2 } else { 1 };
                    self.clip(
                        c,
                        depth,
                        source,
                        next,
                        if through { Some(extra) } else { None },
                    )?;
                    source = next;
                }
                self.emit(
                    if through {
                        OperationKind::Influence
                    } else {
                        OperationKind::ApplyPrepared
                    },
                    i,
                    output,
                    source,
                    current,
                    None,
                )?;
            }
            current = output;
            at = end;
        }
        Ok(current)
    }
}
pub(super) fn compile(nodes: &[Node]) -> Result<(Vec<Operation>, u32, u32), PlanError> {
    let mut children = vec![Vec::new(); nodes.len() + 1];
    let mut max_depth = 0;
    for (i, node) in nodes.iter().enumerate() {
        if node.kind == NodeKind::Mask {
            continue;
        }
        children[node.parent.unwrap_or(nodes.len())].push(i);
        if node.visible {
            let mut parent = node.parent;
            let mut depth = u32::from(node.kind == NodeKind::Group);
            while let Some(p) = parent {
                depth += 1;
                parent = nodes[p].parent;
            }
            max_depth = max_depth.max(depth);
        }
    }
    if max_depth > 16 {
        return Err(PlanError::Budget);
    }
    let mut compiler = Compiler {
        nodes,
        children,
        operations: Vec::new(),
        extras: 2 * (max_depth + 1),
        surfaces: 2,
    };
    compiler.emit(OperationKind::Clear, 0, 0, 1, 1, None)?;
    let output = compiler.stack(nodes.len(), 0, 0)?;
    if compiler.surfaces > 85 {
        return Err(PlanError::Budget);
    }
    Ok((compiler.operations, compiler.surfaces, output))
}
