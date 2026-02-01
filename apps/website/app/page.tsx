import { Navbar } from "@/components/navbar"
import { HeroSection } from "@/components/hero-section"
import { FeatureGrid } from "@/components/feature-grid"
import { CLIFooter } from "@/components/cli-footer"

export default function Home() {
  return (
    <main className="min-h-screen bg-void">
      <Navbar />
      <HeroSection />
      <FeatureGrid />
      <CLIFooter />
    </main>
  )
}
