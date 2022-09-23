# Why Pyrsia is being built in Rust?

When we started working on Pyrsia we had the difficult and exciting task of choosing a language that would work for building secure supply chain software. We will share some details of how we decided to build Pyrsia in Rust and how it fits the problem we are set to solve.

## Securing Open Source Software

Open source software is built mostly by people passionate about solving a problem and sharing their solutions widely. One of the differences in how we build proprietary software vs open source is that, we find that many of those original developers do their best in keeping the software uptodate and try to patch vulnerabilities as soon as they can. Although their efforts are usually the best they can do. In some well publicized cases, developers have experienced burnout and lack of interest in supporting what they built, due to resource constraints.

For proprietary software there are well published processes and patterns which are used to build software and record how it was done. Open Source software does not get this rigor and often is found vulnerable.

On Pyrsia we focus on this missing piece of building open source software and are building a platform that offers build from source service while providing a record of how it was done. Pyrsia leverages a peer-to-peer distribution model for these binaries thus making the network resilient to failures.

### Secure Software needs a secure platform

While the aim of Pyrsia is to secure the software that it builds, a lot of trust/community involvement expects the network itself to be secure. Pyrsia as a platform has taken this expectation seriously and from the initial days invested a lot of energy into building it right.

A few other considerations that were made during the initial discussions of Pyrsia include

* Decentralized network (think Web3) to leverage distribution of binaries across regions
* Build from source using independent randomly chosen nodes to ensure security by reducing surface of attack
* Consensus mechanism to ensure that multiple nodes participate in the build and verification process
* Deploying Pyrsia node instances on wide variety of architectures, operating systems, as well as footprints(think Intel Xeon all the way to Raspberry * Pi and beyond)
* Ensuring wide deployments have a minimal footprint - for transportation, but more importantly to further reduce the possibility of attacks
* Modern software that allows system programming - to enhance experience and also to restrict how the data structures can be used. Constraining how the software is built is key to making it more secure.

## Choosing a programming language

For Pyrsia to address the above considerations we weighed them against a few popular languages with decent community voice and size.

Specifically we were looking for the following in a programming language ecosystem:

* Welcoming community - People come first
* Modern language constructs that help us focus on the problem instead of the language
* Secure or easily to build for security
* Multiple OS and Arch support
* Ability to drop down to lower level to help implement any cryptography, improve performance
* Support for web3 implementations like p2p networking, blockchain, cryptography
* Installed base of system software in the language

[Rust](http://rustlang.org) seemed to satisfy all these requirements, in fact with flying colors.

## RUST Language

### Rust philosophy [1]

> Today we are very proud to announce the 1.0 release of Rust, a new programming language aiming to make it easier to build reliable, efficient systems. Rust combines low-level control over performance with high-level convenience and safety guarantees. Better yet, it achieves these goals without requiring a garbage collector or runtime, making it possible to use Rust libraries as a "drop-in replacement" for C.

What makes Rust different from other languages is its type system, which represents a refinement and codification of "best practices" that have been hammered out by generations of C and C++ programmers. As such, Rust has something to offer for both experienced systems programmers and newcomers alike: experienced programmers will find they save time they would have spent debugging, whereas newcomers can write low-level code without worrying about minor mistakes leading to mysterious crashes.

### History of Rust [2]

> Rust began as a side project of Graydon Hoare, an employee at Mozilla. In short order, Mozilla saw the potential of the new language and began sponsoring it, before revealing it to the world in 2010.
One possible source of the name, according to Hoare, is the rust fungus. This has caused Rust programmers to adopt “Rustaceans” as their moniker of choice.
>
> Despite its relative youth, Rust has steadily risen in the ranks of popular programming languages. In fact, while it ranked 33 in July 2019, by July 2020 it had risen to the 18th spot on the [TIOBE Programming Community Index](<https://www.tiobe.com/tiobe-index/>). Similarly, according to [Stack Overflow Developer Survey](https://insights.stackoverflow.com/survey/2020#technology-most-loved-dreaded-and-wanted-languages-loved), Rust has been the “most loved” language since 2016.

### Rust language ecosystem

The above philosophy made Rust a great candidate for use in security solutions like Pyrsia. Some other aspects that sealed the deal as a programming language for us were:

* Performance close to quivalent C level programs [3]
* Concurrent programming without the garbage collection [4]
* Rust has a borrow checker which ensures references do not outlive the data
* Rust can be compiled to reduced instruction set architectures

Along with the above we also found that the initial set of libraries(libp2p, AlephBFT) we were looking to support had mature implementations in rust. Also we found that the communities that supported these libraries were welcoming all implementers and learners alike. This openness within the rust community in general made the choice easier for us.
A lot of these appealing features of the Rust ecosystem are well summarized in [5].

## Summary

When we set out to change how open source software is secured we had a choice to make - the language to build the security solution with. When we surveyed what was available there were multiple options. C due to its performance, Golang because of its mature installations in the wild, Rust as an up and coming community with performance and modern language features.

In the end it was clear to us that Rust was the right choice to make and we have started building Pyrsia in Rust. We realize that we have a steep learning curve and we are learning as a group. Come join us on our slack channel to discuss more.

### References

* [Rust Philosophy](https://blog.rust-lang.org/2015/05/15/Rust-1.0.html)
* [Rust History](https://www.talentopia.com/news/the-rust-programming-language-its-history-and-why/)
* [Rust vs C](https://codilime.com/blog/rust-vs-c-safety-and-performance-in-low-level-network-programming/)
* [Garbage collection issues](https://discord.com/blog/why-discord-is-switching-from-go-to-rust)
* [Why projects use RUST?](https://codilime.com/blog/why-is-rust-programming-language-so-popular/#:~:text=High%20performance%20and%20safety%20are,amounts%20of%20data%20very%20quickly)
