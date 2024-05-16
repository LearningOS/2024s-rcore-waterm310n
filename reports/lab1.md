# Lab1 总结

## 功能实现总结

首先分析要实现的功能：获取任务信息。
分别需要获取到其中的任务状态，系统调用次数，执行时间三个信息。

要获取这些信息，就必须要知道这些信息存储在什么地方。而Task是由全局变量TASK_MANAGER管理，其中的TaskManagerInner使用TaskControlBlock数组保存了Task的信息。而TaskControlBlock中已经记录了任务状态与任务的上下文。所以我们要做的就是，让为TaskControlBlock添加字段，能够记录Task的系统调用次数，首次执行时间。

然后接下来就是要思考，什么时候应该更新TaskControlBlock中的数据。

显然，对于系统调用次数的统计，应该在每一次系统调用时更新。因此我在syscall函数中，在每次接收到系统调用时，根据当前的任务，更新它的TaskControlBlock中的数据。

而对于TaskControlBlock，则应当在Task第一次运行的时候记录时间。仔细检查TaskManager，可以发现只需在run_first_task与run_next_task调用的时候，才可能有新的Task被调用，因此只需要在这两种情况下，判断是否第一次调用，如果是，就记录调用时间即可。

## 问答题

### 1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

rcore会报无效指令错误，然后内核将程序kill，回收内存
```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003ac, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

### 2. 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:1\) L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。2\)L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
1）刚进入__restore的时候，a0代表内核栈栈顶指针。使用场景有：
- 应用程序陷入内核后，内核执行完异常后，返回原程序。
- 内核运行第一个程序时，通过push一个trapcontext，然后通过__restore运行第一个程序
2) 特殊处理了用户栈指针，前一个上下文所处的特权状态，返回后要执行的指令地址。
sstatus用于将状态特权，转换为陷入内核的前一个状态，通常是用户态。
sscratch用户栈指针的意义是使用用户栈
spec被用于访问下一条指令，所以从保存的上下文中取出下一条要执行的用户程序的指令地址。
```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

### 3. L50-L56：为何跳过了 x2 和 x4？
因为x2是x2已经在上面的指令中读取过并恢复了，
x4是线程指针（用于线程本地存储），可能现在还没有涉及线程，所以用不着这个？
```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

### 4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
sp指向用户栈
sscratch指向内核栈
```asm
csrrw sp, sscratch, sp
```
### 5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
L49 csrw sstatus, t0
因为t0存的值是00，00表示U特权

### 6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
sp指向内核栈
sscratch指向用户栈
```asm
csrrw sp, sscratch, sp
```

### 7.从 U 态进入 S 态是哪一条指令发生的？
一开始陷入内核的时候就进入了S态，大概是__alltraps的调用？

## 荣誉准则
在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

## 看法
改善一下工作流程会比较好？现在本地ci-user会破坏build.rs和makefile，需要手动撤销更改，比较麻烦。