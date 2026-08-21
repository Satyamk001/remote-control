"use client"
import { useEffect, useRef, useState } from "react"
import { AnimatePresence, motion, type Variants } from "framer-motion"
import { cn } from "@/lib/utils"

interface HyperTextProps {
    text: string
    duration?: number
    framerProps?: Variants
    className?: string
    animateOnLoad?: boolean
}

const alphabets = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".split("")

const getRandomInt = (max: number) => Math.floor(Math.random() * max)

export function HyperText({
    text,
    duration = 600,
    framerProps = {
        initial: { opacity: 0, y: -5 },
        animate: { opacity: 1, y: 0 },
        exit: { opacity: 0, y: 3 },
    },
    className,
    animateOnLoad = true,
}: HyperTextProps) {
    const [displayText, setDisplayText] = useState(text.split(""))
    const [trigger, setTrigger] = useState(false)
    const iterations = useRef(0)
    const isFirstRender = useRef(true)

    const triggerAnimation = () => {
        iterations.current = 0
        setTrigger(true)
    }

    useEffect(() => {
        const interval = setInterval(() => {
            if (!trigger) {
                clearInterval(interval)
                return
            }
            if (iterations.current < text.length) {
                setDisplayText((t) =>
                    t.map((l, i) =>
                        l === " "
                            ? l
                            : i <= iterations.current
                                ? text[i]
                                : alphabets[getRandomInt(alphabets.length)]
                    )
                )
                iterations.current = iterations.current + 0.15
            } else {
                setTrigger(false)
                clearInterval(interval)
            }
        }, duration / (text.length * 8))
        return () => clearInterval(interval)
    }, [text, duration, trigger, animateOnLoad])

    useEffect(() => {
        if (!animateOnLoad && isFirstRender.current) {
            isFirstRender.current = false
            return
        }
        triggerAnimation()
    }, [text])

    return (
        <div
            className="overflow-hidden flex cursor-default select-none items-center"
            onMouseEnter={triggerAnimation}
        >
            <AnimatePresence mode="wait">
                {displayText.map((letter, i) => (
                    <motion.span
                        key={i}
                        className={cn("inline-block", letter === " " ? "w-2" : "", className)}
                        {...framerProps}
                    >
                        {letter}
                    </motion.span>
                ))}
            </AnimatePresence>
        </div>
    )
}
